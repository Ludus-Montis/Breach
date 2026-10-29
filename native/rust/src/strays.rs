//! A unit on its own makes for home (FormationStrays, Decision 126; spec 39 step 5b): one
//! that came to after being downed, its formation gone or out of sight, or one bearing the
//! wounded home. It becomes a lone router of its own (FormationRout) fleeing home along its
//! formation's route from where it stands: caught by a friendly formation it runs into, it
//! joins it; struck, it may be downed again or surrender; home, it returns to the reserve.
//! Strays setting off together are numbered in their units' seeded draws' order, never the
//! list's (Decision 97): a squad's id salts its own draws. Since spec 39 step 8b5-xxvi the
//! crate makes the strays' squads too (`adopt`), and GDScript wraps each slot it opened
//! (FormationStrays.adopt).

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::contest;
use crate::entries::{self, Key, Value};
use crate::field::Field;
use crate::joins;
use crate::locks::ROUTING;
use crate::route::Route;
use crate::squads::Squads;
use crate::turn;

/// FormationStrays.strand's setting off: the unit (`unit`: its id and store slot), alone in
/// the stray squad in squad slot `stray`, flees home along `route` from where it stands: it
/// walks home straight, not fanning out as a panicked rout does (RoutFlight), in its squad's
/// only place. Returns the squad's front (tiles along the route): where the unit stands.
pub fn set_off(
    squads: &mut Squads,
    columns: &mut Columns,
    stray: usize,
    unit: (i64, usize),
    route: &Route,
) -> f64 {
    let at = columns.position[unit.1];
    let along = route.distance_of(at);
    let offset = at.sub(route.point_at(along));
    entries::flee(squads, columns, stray, unit, along, offset);
    entries::keep(columns, unit.1, Key::Fanned, Value::Real(f64::INFINITY));
    columns.rank[unit.1] = 0;
    columns.column[unit.1] = 0;
    along / CELLS_PER_TILE as f64
}

/// FormationStrays.adopt's order: the indices of the strays' units (`ids`) in the order their
/// squads are numbered, by each unit's draw under `seed`.
pub fn numbered(ids: &[i64], seed: i64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..ids.len()).collect();
    order.sort_by_key(|&n| contest::draw(seed, ids[n]));
    order
}

/// FormationStrays.adopt: each of `strays` (its squad's slot, its unit's store slot) set off
/// home alone, as a squad of its own the crate opens, numbered from the ids the battle issues
/// in `numbered`'s order under `seed`. The unit leaves its squad (its loose, fleeing and
/// chasing entries with it); its squad takes its old squad's faction, way, home, command,
/// route and heading, ROUTING with no morale, and its front where the unit stands. Returns
/// each stray's index in `strays` and the slot opened for it, in the order numbered.
pub fn adopt(
    squads: &mut Squads,
    field: &mut Field,
    strays: &[(usize, usize)],
    seed: i64,
) -> Vec<(usize, usize)> {
    let ids: Vec<i64> = strays.iter().map(|&(_, u)| field.unit_at(u as u32).unwrap_or(0)).collect();
    let mut opened = Vec::with_capacity(strays.len());
    for n in numbered(&ids, seed) {
        let (own, slot) = strays[n];
        let unit = (ids[n], slot);
        squads.members[own].retain(|&u| u as usize != slot);
        entries::unloose(squads, &mut field.columns, own, unit);
        entries::unflee(squads, &mut field.columns, own, unit);
        squads.chasers[own].retain(|c| c.unit != unit.0);
        let id = squads.issue_squad();
        let stray = squads.open(id) as usize;
        squads.faction[stray] = squads.faction[own];
        squads.direction[stray] = squads.direction[own];
        squads.home_distance[stray] = squads.home_distance[own];
        squads.width[stray] = 1;
        squads.command[stray] = squads.command[own];
        joins::enlist_one(squads, field, stray, slot as i32);
        let route = squads.route[own].clone().expect("a stray flees along its squad's route");
        squads.heading[stray] = squads.heading[own];
        squads.state[stray] = ROUTING;
        squads.morale[stray] = 0;
        let front = set_off(squads, &mut field.columns, stray, unit, &route);
        squads.front_distance[stray] = front;
        squads.position[stray] = turn::place(Some(&route), front * CELLS_PER_TILE as f64);
        squads.route[stray] = Some(route);
        opened.push((n, stray));
    }
    opened
}

#[cfg(test)]
mod tests;
