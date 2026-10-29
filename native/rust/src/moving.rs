//! The move phase's choices (FormationSimulation._move and _march, spec 39 step 8b1): which
//! squads march, hold or wait as the phase begins, whether one that may march holds where it
//! stands, and every move landed together, each advancing squad stopped at contact or behind
//! a friend as they stood at the tick's start (Decision 97). The pass (`pass`, spec 39 step
//! 8b5-viii) runs each march between them (staging, the turn, the sweep, the pace, the drag)
//! and the ends after; the walk's share and the joins are GDScript's still.

pub mod pass;
#[cfg(test)]
mod tests;

use crate::columns::Columns;
use crate::contact::Field;
use crate::locks::{ARRIVED, DESTROYED, FIGHTING, HOLDING, MOVING, ROUTING};
use crate::march::ADVANCE;
use crate::roster::Roster;
use crate::squads::Squads;

/// SkirmishUnit.Order's hold.
const HOLD: i32 = 1;

/// What a squad does as the move phase begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    /// Fighting, routing, arrived or destroyed, or regrouping: it doesn't march.
    Stands = 0,
    /// Still waiting to set out: its wait runs down once the moves have landed.
    Waits = 1,
    /// It may march, unless it holds.
    Goes = 2,
}

/// FormationSimulation._move's first choice for the squad in `slot`.
pub fn choose(squads: &Squads, slot: usize) -> Choice {
    if matches!(squads.state[slot], DESTROYED | FIGHTING | ROUTING | ARRIVED)
        || regrouping(squads, slot)
    {
        Choice::Stands
    } else if squads.wait_ticks[slot] > 0 {
        Choice::Waits
    } else {
        Choice::Goes
    }
}

/// FormationScrum.regrouping: true while any of the squad's loose units is loose for a
/// reason other than a chase of its own (Decision 112).
pub fn regrouping(squads: &Squads, slot: usize) -> bool {
    squads.loose[slot].iter().any(|&unit| !squads.chasing(slot, unit))
}

/// Whether `roster`'s squad `listed`, free to march, holds where it stands: ordered to
/// hold, or skirmishing (Decision 47). Its state becomes holding or moving.
pub fn hold(squads: &mut Squads, columns: &Columns, roster: &Roster, listed: usize) -> bool {
    let slot = roster.listed[listed].squad;
    let holds = squads.order[slot] == HOLD || Field { squads, columns, roster }.skirmishing(listed);
    squads.state[slot] = if holds { HOLDING } else { MOVING };
    holds
}

/// A squad's march this tick: its index in the roster and where it wants to get (tiles
/// along its route).
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub listed: usize,
    pub next: f64,
}

/// FormationSimulation._march's landing: each plan's front as it lands, in order, and
/// whether it advances. An advancing squad's limit (FormationContact.limit) is judged
/// before any move lands, with every advancing squad marching.
pub fn land(field: &Field, plans: &[Plan]) -> Vec<(f64, bool)> {
    let advances = |p: &Plan| field.squads.order[field.roster.listed[p.listed].squad] == ADVANCE;
    let marching: Vec<i64> =
        plans.iter().filter(|p| advances(p)).map(|p| field.roster.listed[p.listed].id).collect();
    plans
        .iter()
        .map(|p| {
            if advances(p) {
                (field.limit(p.listed, p.next, &marching), true)
            } else {
                (p.next, false)
            }
        })
        .collect()
}
