//! Taking the downed (FormationTaking, spec 30 round 3 part 8, Decisions 121 and 124; spec 39
//! step 6c): which free unit of a group out of a fight walks out to which downed foe. Each
//! free unit is paired with a downed foe it can see within `reach`, one unit to a body,
//! nearest first, ties by the unit's then the body's seeded draws - never ids or lists
//! (Decision 97). Which squads send, the hold, the recall and the events are the pass's
//! (`pass`, spec 39 step 8b5-xxiii).

use crate::columns::Columns;
use crate::contest;
use crate::maths::snapped;

/// A unit as the takings are given it: its id, store slot, faction (a code) and, for a
/// taker, how far it sees.
#[derive(Clone, Debug)]
pub struct Taker {
    pub id: i64,
    pub slot: usize,
    pub faction: u32,
    pub detection: f64,
}

/// A squad that sends takers out: its free units (in their places, not loose, fleeing or
/// out already) and the bodies its takers have claimed (by index in the downed).
#[derive(Clone, Debug, Default)]
pub struct Sending {
    pub free: Vec<Taker>,
    pub claimed: Vec<usize>,
}

/// Each sending squad's takers sent out this tick, in the order they were: [the unit's index
/// in its free units, the body's index in `downed`]. `downed` are the bodies lying, not
/// borne, of every squad.
pub fn send(
    columns: &Columns,
    squads: &[Sending],
    downed: &[Taker],
    seed: i64,
    reach: f64,
) -> Vec<Vec<(usize, usize)>> {
    squads.iter().map(|squad| sent(columns, squad, downed, seed, reach)).collect()
}

/// One squad's takers sent out.
fn sent(
    columns: &Columns,
    squad: &Sending,
    downed: &[Taker],
    seed: i64,
    reach: f64,
) -> Vec<(usize, usize)> {
    let mut pairs: Vec<((f64, i64, i64), usize, usize)> = vec![];
    let mut claimed = vec![false; downed.len()];
    for &b in &squad.claimed {
        claimed[b] = true;
    }
    for (u, unit) in squad.free.iter().enumerate() {
        let here = columns.position[unit.slot];
        for (b, body) in downed.iter().enumerate() {
            if body.faction == unit.faction || claimed[b] {
                continue;
            }
            let gap = here.distance_to(columns.position[body.slot]) as f64;
            if gap <= reach && gap <= unit.detection {
                let key = (
                    snapped(gap, 0.000001),
                    contest::draw(seed, unit.id),
                    contest::draw(seed, body.id),
                );
                pairs.push((key, u, b));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("a gap is a number"));
    let mut busy = vec![false; squad.free.len()];
    let mut out = vec![];
    for (_, u, b) in pairs {
        if busy[u] || claimed[b] {
            continue;
        }
        busy[u] = true;
        claimed[b] = true;
        out.push((u, b));
    }
    out
}

pub mod pass;
#[cfg(test)]
mod tests;
