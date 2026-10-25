//! A withdrawal (FormationWithdraw, Decision 99, spec 27 round 10; spec 39 step 4c): ordered
//! to retreat out of a fight, a formation leaves from where its units stand. Each unit turns
//! for home and heads there along its route; until clear of the foes it touches it turns
//! and steps away at its formation's re-form pace (disengaging is a manoeuvre, as re-forming
//! is), then flees at its full pace. The less ordered the formation, the wider its units fan
//! out from the route's line, by a seeded angle per unit; ground it can't cross turns it
//! back onto the route. It is safe with no standing enemy within `rout_enemy_near` of it and
//! none pursuing it. Who is engaged and who is safe is decided from where everyone stood
//! before anyone fled (Decision 97). Its record is the ticks it has felt safe (spec 39 step
//! 7c): safe for `rout_rally_seconds`, or with all its units home, it re-forms on its route
//! where its units stand (`reform`), and GDScript turns it to face home (ScrumTurn) and lands
//! the events.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::ground::Ground;
use crate::locks;
use crate::maths::{clampf, is_zero_approx, maxf, minf, move_toward, rotated, signf, V2};
use crate::mover;
use crate::near::Foes;
use crate::roster::{self, Member, Roster};
use crate::route::Route;
use crate::squads::Squads;

/// How near (cells, along its route) home a unit has nowhere further to go.
const HOME_CELLS: f64 = 0.5;
/// How far ahead along its route (cells) a fleeing unit looks for the way home.
const LOOK_CELLS: f64 = 1.0;
const EPSILON: f64 = 0.000001;
const CELLS: f64 = CELLS_PER_TILE as f64;
/// SkirmishUnit.Order's hold.
const HOLD: i32 = 1;

/// A withdrawing squad of the roster (not routing or destroyed), in the lists' order: its
/// index in the roster, whether a squad pursues it, how far from orderly it is (0 drilled
/// to 1 no discipline), its re-form pace, its home along its route in cells, its route, and
/// each living unit's own flight angle (radians, at full disorder: RoutFlight.fan), in its
/// order.
pub struct Withdrawing {
    pub listed: usize,
    pub pursued: bool,
    pub disorder: f64,
    pub reform: f64,
    pub home: f64,
    pub route: Route,
    pub fans: Vec<f64>,
}

/// The tick's numbers: the march pace (cells a tick at speed 1), the tick's seconds,
/// `reach_contact`, `rout_enemy_near` and `rout_fan_cells`.
pub struct Rules {
    pub pace: f64,
    pub seconds: f64,
    pub reach: f64,
    pub near: f64,
    pub fan_cells: f64,
}

/// FormationWithdraw.step's flight: every withdrawing squad's units flee. Returns, for each,
/// whether it was safe as the tick began and whether all its units are home after.
pub fn step(
    squads: &Squads,
    columns: &mut Columns,
    roster: &Roster,
    withdrawing: &[Withdrawing],
    ground: Option<&Ground>,
    rules: &Rules,
) -> Vec<(bool, bool)> {
    let mut hostile: Vec<(u32, Foes)> = Vec::new(); // faction -> its foes, built once a pass
    let mut decided = Vec::with_capacity(withdrawing.len());
    for w in withdrawing {
        let listed = &roster.listed[w.listed];
        let found = match hostile.iter().position(|(f, _)| *f == listed.faction) {
            Some(k) => k,
            None => {
                hostile.push((listed.faction, Foes::of(roster, squads, columns, listed.faction)));
                hostile.len() - 1
            }
        };
        let foes = &hostile[found].1;
        let loose: HashSet<i64> = squads.loose[listed.squad].iter().copied().collect();
        let engaged: Vec<bool> = listed
            .living
            .iter()
            .map(|unit| {
                let at = roster::at(columns, &loose, unit);
                let close = foes.around(at, unit.radius() + rules.reach);
                foes.touches_any(roster, unit, at, &close, rules.reach)
            })
            .collect();
        let safe = !w.pursued && safe(columns, &loose, &listed.living, foes, rules.near);
        decided.push((safe, engaged));
    }
    let mut out = Vec::with_capacity(withdrawing.len());
    for (w, (safe, engaged)) in withdrawing.iter().zip(decided) {
        let listed = &roster.listed[w.listed];
        for (u, unit) in listed.living.iter().enumerate() {
            let full = rules.pace * columns.speed[unit.slot];
            let fleeing = Flight { full, disengaging: engaged[u], fan: w.fans[u] };
            flee(columns, unit, w, &fleeing, ground, rules);
        }
        out.push((safe, home(squads, columns, roster, w.listed, &w.route, w.home)));
    }
    out
}

/// One unit's flight this tick: its pace (cells a tick, its speed in), whether it is still
/// disengaging, and its own flight angle.
struct Flight {
    full: f64,
    disengaging: bool,
    fan: f64,
}

/// FormationWithdraw._flee: the unit steps for home along its route, fanned out by its
/// squad's disorder, turning and stepping at its re-form pace until it is clear.
fn flee(
    columns: &mut Columns,
    unit: &Member,
    w: &Withdrawing,
    flight: &Flight,
    ground: Option<&Ground>,
    rules: &Rules,
) {
    let slot = unit.slot;
    let at = columns.loose_at[slot];
    let route = &w.route;
    let along = route.distance_of(at);
    if (along - w.home).abs() < HOME_CELLS {
        return; // home
    }
    let homeward = homeward(route, along, w.home);
    let full = flight.full;
    let mut heading = rotated(homeward, flight.fan * w.disorder);
    let side = homeward.orthogonal();
    let aside = at.sub(route.point_at(along)).dot(side) as f64;
    if aside.abs() >= rules.fan_cells * w.disorder && heading.dot(side) as f64 * aside > 0.0 {
        heading = homeward; // fanned out as far as its disorder takes it (a rout's at most)
    }
    let reach = maxf(full, 1.0); // where its step lands, a run's longer than a cell
    if let Some(g) = ground {
        if g.crossing(&unit.walker, at, at.add(heading.scale(reach))) <= 0.0 {
            let onto = route.point_at(move_toward(along, w.home, LOOK_CELLS)).sub(at);
            heading = if onto.length() as f64 > EPSILON { onto.normalized() } else { homeward };
        }
    }
    let to = at.add(heading.scale(maxf(full, EPSILON)));
    let turning = rules.seconds * w.reform; // a turn is a re-form
    let step = if flight.disengaging { full * minf(w.reform, 1.0) } else { full };
    let mover = unit.mover(columns);
    let (stepped, bearing) = mover::step(ground, &unit.walker, &mover, at, to, step, turning, None);
    columns.bearing[slot] = bearing;
    columns.loose_at[slot] = stepped;
    columns.next[slot] = stepped;
    columns.goal_foe[slot] = 0;
    columns.goal_slot[slot] = -1;
}

/// FormationWithdraw._homeward: the way home along the route from `along`, towards the route
/// a step nearer home, so a unit at a bend turns with the route.
fn homeward(route: &Route, along: f64, home: f64) -> V2 {
    let nearer = move_toward(along, home, LOOK_CELLS);
    let way = route.point_at(nearer).sub(route.point_at(along));
    if (way.length() as f64) < EPSILON {
        return route.heading_at(along).scale(signf(home - along));
    }
    way.normalized()
}

/// FormationWithdraw._safe's look round: true if no unit of `foes` stands within `near` of
/// any of `living`.
fn safe(
    columns: &Columns,
    loose: &HashSet<i64>,
    living: &[Member],
    foes: &Foes,
    near: f64,
) -> bool {
    living.iter().all(|unit| {
        let at = roster::at(columns, loose, unit);
        foes.near(at, near + crate::grid::MARGIN)
            .iter()
            .all(|&f| foes.points[f].distance_to(at) as f64 > near)
    })
}

/// FormationWithdraw._home: true if all the listed squad's units stand within HOME_CELLS of
/// `home` (cells) along `route`: it has nowhere further to go.
pub fn home(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    n: usize,
    route: &Route,
    home: f64,
) -> bool {
    let listed = &roster.listed[n];
    let loose: HashSet<i64> = squads.loose[listed.squad].iter().copied().collect();
    listed.living.iter().all(|unit| {
        (route.distance_of(roster::at(columns, &loose, unit)) - home).abs() < HOME_CELLS
    })
}

/// A withdrawal that re-formed, by the squad's index in the roster: the way along its route it
/// now faces (+1 up it, -1 down it), and whether it is home with nowhere further to go (it
/// then holds there, facing out).
#[derive(Debug, PartialEq)]
pub struct Reformed {
    pub listed: usize,
    pub homeward: i32,
    pub arrived: bool,
}

/// The first part of FormationWithdraw.step: a listed squad routing or destroyed withdraws no
/// more.
pub fn ended(squads: &mut Squads, roster: &Roster) {
    for listed in &roster.listed {
        let s = listed.squad;
        if matches!(squads.state[s], locks::ROUTING | locks::DESTROYED) {
            squads.withdraw[s] = None;
        }
    }
}

/// FormationWithdraw._pursued: true if a listed squad pursues the listed `n`, not yet on its
/// way back.
pub fn pursued(squads: &Squads, roster: &Roster, n: usize) -> bool {
    let id = roster.listed[n].id;
    roster
        .listed
        .iter()
        .any(|other| squads.pursuit[other.squad].is_some_and(|p| p.foe == id && !p.returning))
}

/// The rest of FormationWithdraw.step: each withdrawing squad, in order, counts the ticks it
/// has been safe (`out`: what `step` returned for it), and re-forms once that is
/// `rally_seconds` of `seconds` ticks or it is home.
pub fn records(
    squads: &mut Squads,
    columns: &Columns,
    roster: &Roster,
    withdrawing: &[Withdrawing],
    out: &[(bool, bool)],
    (seconds, rally_seconds): (f64, f64),
) -> Vec<Reformed> {
    let mut reformed = Vec::new();
    for (w, &(safe, home)) in withdrawing.iter().zip(out) {
        let s = roster.listed[w.listed].squad;
        let ticks = if safe { squads.withdraw[s].unwrap_or(0) + 1 } else { 0 };
        squads.withdraw[s] = Some(ticks);
        if ticks as f64 * seconds >= rally_seconds || home {
            reformed.push(reform(squads, columns, roster, w));
        }
    }
    reformed
}

/// FormationWithdraw._reform_here: puts the squad's frame on its route where its units stand
/// (their mean along it, kept between its home and its front), its withdrawal over, facing
/// home; one whose units are all home goes to its home, to face out and hold.
fn reform(squads: &mut Squads, columns: &Columns, roster: &Roster, w: &Withdrawing) -> Reformed {
    let listed = &roster.listed[w.listed];
    let s = listed.squad;
    let loose: HashSet<i64> = squads.loose[s].iter().copied().collect();
    let mut total = 0.0;
    for unit in &listed.living {
        total += w.route.distance_of(roster::at(columns, &loose, unit));
    }
    let mean = total / listed.living.len().max(1) as f64 / CELLS;
    let (home, front) = (squads.home_distance[s], squads.front_distance[s]);
    squads.front_distance[s] = clampf(mean, minf(home, front), maxf(home, front));
    squads.withdraw[s] = None;
    let mut homeward = if home < squads.front_distance[s] { -1 } else { 1 };
    let arrived = self::home(squads, columns, roster, w.listed, &w.route, w.home);
    if arrived {
        homeward = if is_zero_approx(home) { 1 } else { -1 };
        squads.front_distance[s] = home;
        squads.order[s] = HOLD;
    }
    squads.position[s] = w.route.point_at(squads.front_distance[s] * CELLS);
    Reformed { listed: w.listed, homeward, arrived }
}

#[cfg(test)]
#[path = "withdraw/tests.rs"]
mod tests;
