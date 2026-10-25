//! Orders (FormationSimulation._apply_orders and ScrumPursuit.retreat, Decisions 95, 99 and
//! 109; spec 39 step 7c): the orders given since the last tick apply together, each to its
//! squad if it stands, in the order they were given. Who is fighting is read before any
//! lands, so orders given together act together. A squad ordered to retreat out of a fight
//! retreats: its locks end at once and it withdraws, its units leaving their places where
//! they stand (FormationWithdraw.begin); each enemy fighting it that may pursue locks onto it
//! and pursues it as a whole along the route it flees by (FormationPursuit.begin, Decision
//! 113), and each enemy's units near it may break ranks to chase it (`pursuit::tempt`). The
//! morale shock of a ragged retreat and the events are GDScript's (FormationOrders), landed
//! from what this returns.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::entries;
use crate::locks;
use crate::maths::V2;
use crate::pursuit::{self, Temptation};
use crate::roster::Roster;
use crate::route::Route;
use crate::squads::{Pursuit, Squads};

const CELLS: f64 = CELLS_PER_TILE as f64;
/// A listed squad's route as a retreat reads it: its code and its route (none: the lane).
pub type Coded = (i64, Option<Route>);
/// How near (cells) a route must pass a pursuer for it to join it (FormationPursuit).
const JOIN_CELLS: f64 = 2.0;

/// What a retreat reads of every listed squad, by its index in the roster: its route (none:
/// the straight lane) and a code naming it, the same for one route; whether it may pursue;
/// and its living units' chance to break ranks and leash (FormationDiscipline), flat in the
/// roster's order. Then `pursuit_tempted_within`, the tick and the battle's seed. A pursuer
/// that takes its quarry's route has it from then on, for a later retreat of the tick.
pub struct Retreats<'a> {
    pub routes: Vec<Coded>,
    pub pursues: &'a [bool],
    pub temptations: &'a [(f64, f64)],
    pub within: f64,
    pub tick: i64,
    pub seed: i64,
}

/// An order that applied: the squad's index in the roster, and if it retreated out of a
/// fight, the enemies that began to pursue it, in the lists' order.
#[derive(Debug, PartialEq)]
pub struct Applied {
    pub listed: usize,
    pub retreat: Option<Vec<Pursuer>>,
}

/// An enemy that pursues a retreating squad: its index in the roster, whether its pursuit
/// began now (it had none) and whether it took its quarry's route.
#[derive(Debug, PartialEq)]
pub struct Pursuer {
    pub listed: usize,
    pub began: bool,
    pub took: bool,
}

/// FormationSimulation._apply_orders: `given` [(squad id, order)] in the order given. A
/// squad not listed or destroyed takes none.
pub fn apply(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    given: &[(i64, i32)],
    retreats: &mut Retreats,
) -> Vec<Applied> {
    let find = |id: i64| roster.listed.iter().position(|l| l.id == id);
    let fighting: Vec<bool> = given
        .iter()
        .map(|&(id, _)| {
            find(id).is_some_and(|n| {
                let s = roster.listed[n].squad;
                squads.engaged_with[s] != 0 || !squads.flanks[s].is_empty()
            })
        })
        .collect();
    let mut out = Vec::new();
    for (&(id, order), &fought) in given.iter().zip(&fighting) {
        let Some(n) = find(id) else { continue };
        let s = roster.listed[n].squad;
        if squads.state[s] == locks::DESTROYED {
            continue;
        }
        squads.order[s] = order;
        let retreat = (order == locks::RETREAT && fought)
            .then(|| retreat(squads, columns, roster, n, retreats));
        out.push(Applied { listed: n, retreat });
    }
    out
}

/// ScrumPursuit.retreat: the listed squad `n` ends its fight and withdraws, and its enemies
/// pursue or chase it. Returns those that pursue.
fn retreat(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    n: usize,
    retreats: &mut Retreats,
) -> Vec<Pursuer> {
    let enemies: Vec<usize> =
        (0..roster.listed.len()).filter(|&o| fights(squads, roster, o, n)).collect();
    let slots: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    locks::release(squads, roster.listed[n].squad, &slots);
    begin_withdraw(squads, columns, roster, n);
    let mut starts = Vec::with_capacity(roster.listed.len());
    let mut at = 0;
    for listed in &roster.listed {
        starts.push(at);
        at += listed.living.len();
    }
    let mut out = Vec::new();
    for enemy in enemies {
        if retreats.pursues[enemy] {
            out.push(begin_pursuit(squads, columns, roster, (enemy, n), retreats));
        }
        let living = roster.listed[enemy].living.len();
        let temptation = Temptation {
            chances: &retreats.temptations[starts[enemy]..starts[enemy] + living],
            within: retreats.within,
            tick: retreats.tick,
            seed: retreats.seed,
        };
        pursuit::tempt(squads, columns, roster, (enemy, n), &temptation);
    }
    out
}

/// ScrumPursuit._fights: true if the listed `other` is of another faction and fights the
/// listed `n`, front to front or on an edge of either.
fn fights(squads: &Squads, roster: &Roster, other: usize, n: usize) -> bool {
    let (them, us) = (&roster.listed[other], &roster.listed[n]);
    if them.faction == us.faction {
        return false;
    }
    squads.engaged_with[them.squad] == us.id
        || squads.engaged_with[us.squad] == them.id
        || squads.flanked_by(them.squad, us.id)
}

/// FormationWithdraw.begin: the listed squad `n` withdraws, its stance dropped, its units not
/// yet loose leaving their places where they stand.
pub fn begin_withdraw(squads: &mut Squads, columns: &mut Columns, roster: &Roster, n: usize) {
    let s = roster.listed[n].squad;
    squads.withdraw[s] = Some(0);
    squads.stance[s] = None;
    let loose: HashSet<i64> = squads.loose[s].iter().copied().collect();
    for unit in &roster.listed[n].living {
        if !loose.contains(&unit.id) {
            let stood = columns.position[unit.slot];
            entries::loosen(squads, columns, s, (unit.id, unit.slot), stood);
        }
    }
}

/// FormationPursuit.begin: the listed `enemy` locks onto the listed `foe` and, if it isn't
/// pursuing already, keeps its post and takes up the route its quarry flees by.
fn begin_pursuit(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    (enemy, foe): (usize, usize),
    retreats: &mut Retreats,
) -> Pursuer {
    let listed = &roster.listed[enemy];
    let s = listed.squad;
    let living = listed.slots();
    let locking = locks::Locking { squad: s, living: &living, joined: listed.joined };
    locks::lock(squads, columns, &locking, roster.listed[foe].id);
    let began = squads.pursuit[s].is_none();
    let mut took = false;
    if began {
        squads.pursuit[s] = Some(Pursuit {
            foe: roster.listed[foe].id,
            post: squads.front_distance[s],
            post_at: squads.position[s],
            home: squads.home_distance[s],
            direction: squads.direction[s],
            order: squads.order[s],
            returning: false,
        });
        let towards = squads.position[roster.listed[foe].squad];
        took = take(squads, s, (&retreats.routes[enemy], &retreats.routes[foe]), towards);
        if took {
            retreats.routes[enemy] = retreats.routes[foe].clone();
        }
    }
    Pursuer { listed: enemy, began, took }
}

/// FormationPursuit._take: the squad in `s` takes the route `routes.1` from where it stands,
/// travelling towards `towards`, if that route passes by it and isn't its own (`routes.0`).
/// True if it took it.
fn take(squads: &mut Squads, s: usize, (own, theirs): (&Coded, &Coded), towards: V2) -> bool {
    let (Some(_), (code, Some(route))) = (&own.1, theirs) else { return false };
    if *code == own.0 {
        return false;
    }
    let along = route.distance_of(squads.position[s]);
    if route.point_at(along).distance_to(squads.position[s]) as f64 > JOIN_CELLS {
        return false;
    }
    squads.front_distance[s] = along / CELLS;
    squads.position[s] = route.point_at(squads.front_distance[s] * CELLS);
    squads.direction[s] = if route.distance_of(towards) >= along { 1 } else { -1 };
    true
}

#[cfg(test)]
mod tests;
