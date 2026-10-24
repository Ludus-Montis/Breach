//! A unit on its own makes for home (FormationStrays, Decision 126; spec 39 step 5b): one
//! that came to after being downed, its formation gone or out of sight, or one bearing the
//! wounded home. It becomes a lone router of its own (FormationRout) fleeing home along its
//! formation's route from where it stands: caught by a friendly formation it runs into, it
//! joins it; struck, it may be downed again or surrender; home, it returns to the reserve.
//! Strays setting off together are numbered in their units' seeded draws' order, never the
//! list's (Decision 97): a squad's id salts its own draws. Making the stray's squad
//! (SkirmishSquad, its command and orders) is GDScript's.

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::contest;
use crate::entries::{self, Key, Value};
use crate::route::Route;
use crate::squads::Squads;

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

#[cfg(test)]
mod tests;
