//! A formation pursuing as a whole (FormationPursuit, Decisions 95, 103, 107 and 113; spec 39
//! step 7c): locked on its retreating enemy, its frame advances after it along the route its
//! quarry flees by, no faster than its rearmost unit keeps up (`lagging`), closing on where
//! its quarry's units stand (`spread`). When the enemy is out of its sight, gone or no longer
//! retreating, or it has gone as far from its post as its discipline leashes it, or most of
//! it has tired, it gives up: its fight ends and it marches back along the route it is on to
//! the post it held (a march home, with home its post). Back there it takes up its own route,
//! its post and its order again, and GDScript turns it to face the way it held it
//! (ScrumTurn), from what this returns, with the events.

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::locks;
use crate::march::sweep;
use crate::maths::{bearing_vector, clampf, minf, V2};
use crate::perception;
use crate::roster::Roster;
use crate::route::Route;
use crate::squads::Squads;
use crate::stamina;

const CELLS: f64 = CELLS_PER_TILE as f64;
/// It advances while its enemy is more than this many cells ahead of its front.
const CLOSE_ENOUGH: f64 = 1.0;
/// How far its frame may go along no route (tiles).
const NO_END: f64 = 1e9;

/// A pursuing squad of the roster (its record set, a unit living), in the lists' order: its
/// index in the roster, the route it is on and the route it held its post on (none: the
/// straight lane), and its leash (cells from its post; infinite for none,
/// FormationDiscipline.pursuit_leash).
pub struct Pursuing {
    pub listed: usize,
    pub route: Option<Route>,
    pub held: Option<Route>,
    pub leash: f64,
}

/// The tick's numbers: cells a second at speed 1, the tick's seconds, the share of its most
/// stamina below which a unit is too tired (`stamina_give_up`), and how far (cells) its
/// rearmost may lag behind its place (`reach_pursuit_lag`).
pub struct Rules {
    pub cells_per_second: f64,
    pub seconds: f64,
    pub give_up: f64,
    pub lag: f64,
}

/// What a pursuit did that GDScript lands, by the squad's index in the roster.
#[derive(Debug, PartialEq)]
pub enum Pursued {
    /// It gave up and marches back to its post.
    Ended(usize),
    /// It is back at its post, to face `direction` along its own route again.
    Arrived(usize, i32),
}

/// FormationPursuit.step: each pursuer, in the lists' order, arrives back, gives up or
/// advances after its enemy.
pub fn step(
    squads: &mut Squads,
    columns: &Columns,
    roster: &Roster,
    pursuing: &[Pursuing],
    rules: &Rules,
) -> Vec<Pursued> {
    let slots: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let mut out = Vec::new();
    for p in pursuing {
        let s = roster.listed[p.listed].squad;
        let Some(held) = squads.pursuit[s] else { continue };
        if held.returning {
            if squads.order[s] != locks::RETREAT {
                squads.front_distance[s] = held.post;
                squads.position[s] = on(p.held.as_ref(), held.post);
                squads.home_distance[s] = held.home;
                squads.order[s] = held.order;
                squads.pursuit[s] = None;
                out.push(Pursued::Arrived(p.listed, held.direction));
            }
            continue;
        }
        let foe = roster.listed.iter().position(|l| l.id == held.foe);
        if given_up(squads, columns, roster, (p, foe), rules) {
            locks::release(squads, s, &slots);
            let back = match &p.route {
                Some(route) => route.distance_of(held.post_at),
                None => held.post_at.x as f64,
            };
            squads.home_distance[s] = back / CELLS;
            squads.order[s] = locks::RETREAT;
            if let Some(pursuit) = squads.pursuit[s].as_mut() {
                pursuit.returning = true;
            }
            out.push(Pursued::Ended(p.listed));
            continue;
        }
        advance(squads, columns, roster, (p, foe.expect("a foe it pursues")), rules);
    }
    out
}

/// FormationPursuit.reach's test and _given_up: true if its enemy (listed `foe`) is gone,
/// destroyed or not retreating, out of its sight (on open ground) or most of it is too tired,
/// or its frame has gone as far from its post as its leash.
fn given_up(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    (p, foe): (&Pursuing, Option<usize>),
    rules: &Rules,
) -> bool {
    let Some(foe) = foe else { return true };
    let (us, them) = (&roster.listed[p.listed], &roster.listed[foe]);
    if them.living.is_empty() || squads.order[them.squad] != locks::RETREAT {
        return true;
    }
    let living: Vec<(usize, f64)> = us.living.iter().map(|m| (m.slot, m.max_stamina)).collect();
    if !perception::detects(columns, us, them, None)
        || stamina::squad_gives_up(columns, &living, rules.give_up)
    {
        return true;
    }
    let post_at = squads.pursuit[us.squad].expect("a pursuit").post_at;
    squads.position[us.squad].distance_to(post_at) as f64 >= p.leash
}

/// FormationPursuit._advance: its frame sweeps round its route's bends and moves along it
/// towards its enemy (listed `foe`) while the enemy is ahead and none of it lags.
fn advance(
    squads: &mut Squads,
    columns: &Columns,
    roster: &Roster,
    (p, foe): (&Pursuing, usize),
    rules: &Rules,
) {
    let s = roster.listed[p.listed].squad;
    let Some(spread) = super::spread(squads, columns, roster, foe) else { return };
    let ahead = ahead(squads, s, p.route.as_ref(), spread.center());
    if ahead <= CLOSE_ENOUGH || super::lagging(squads, columns, roster, p.listed, rules.lag) {
        return;
    }
    let slowest = roster.listed[p.listed]
        .living
        .iter()
        .fold(f64::INFINITY, |least, m| minf(least, columns.speed[m.slot]));
    let speed = if slowest == f64::INFINITY { 0.0 } else { slowest };
    let pace = speed * rules.cells_per_second;
    if let Some(route) = &p.route {
        let cells = squads.front_distance[s] * CELLS;
        let wanted = sweep::wanted(route, cells, squads.direction[s], squads.heading[s]);
        squads.heading[s] =
            sweep::step(squads.heading[s], wanted, squads.width[s], pace, rules.seconds);
    }
    let tiles = minf(speed * rules.cells_per_second * rules.seconds, ahead - CLOSE_ENOUGH) / CELLS;
    let length = p.route.as_ref().map_or(NO_END, |r| r.length() / CELLS);
    let front = squads.front_distance[s] + squads.direction[s] as f64 * tiles;
    squads.front_distance[s] = clampf(front, 0.0, length);
    squads.position[s] = on(p.route.as_ref(), squads.front_distance[s]);
}

/// FormationPursuit._ahead: cells `at` lies ahead of the squad's front along its route (as
/// the crow flies along its heading, with none).
fn ahead(squads: &Squads, s: usize, route: Option<&Route>, at: V2) -> f64 {
    let Some(route) = route else {
        return at.sub(squads.position[s]).dot(bearing_vector(squads.heading[s])) as f64;
    };
    let own = squads.front_distance[s] * CELLS;
    (route.distance_of(at) - own) * squads.direction[s] as f64
}

/// Where a front `tiles` along the route stands (StoredSquad._place).
fn on(route: Option<&Route>, tiles: f64) -> V2 {
    let cells = tiles * CELLS;
    match route {
        Some(route) => route.point_at(cells),
        None => V2::new(cells as f32, 0.0),
    }
}

#[cfg(test)]
mod tests;
