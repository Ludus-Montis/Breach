//! A withdrawal (FormationWithdraw, Decision 99, spec 27 round 10; spec 39 step 4c): ordered
//! to retreat out of a fight, a formation leaves from where its units stand. Each unit turns
//! for home and heads there along its route; until clear of the foes it touches it turns
//! and steps away at its formation's re-form pace (disengaging is a manoeuvre, as re-forming
//! is), then flees at its full pace. The less ordered the formation, the wider its units fan
//! out from the route's line, by a seeded angle per unit; ground it can't cross turns it
//! back onto the route. It is safe with no standing enemy within `rout_enemy_near` of it and
//! none pursuing it. Who is engaged and who is safe is decided from where everyone stood
//! before anyone fled (Decision 97); the squad's record of how long it has been safe, and its
//! re-forming, are GDScript's (FormationWithdraw) until orders port.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::ground::Ground;
use crate::maths::{maxf, minf, move_toward, rotated, signf, V2};
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

#[cfg(test)]
#[path = "withdraw/tests.rs"]
mod tests;
