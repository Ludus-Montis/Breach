//! The fallen (FormationDeaths, specs/22-formation-feel-test.md, Decisions 47, 82 and 121;
//! spec 39 step 6b): a unit brought to 0 hp is downed - its hp stops at 0, the excess lost,
//! and it lies where it fell - unless the blows took it past minus its constitution, which
//! kills it outright. A downed unit takes the wounds its last blow would add and has yet to
//! reckon how long it lies. GDScript takes the fell queue, finds each fallen unit's squad,
//! and keeps the units' states, wounds, the morale, the step-ups and the events.

use crate::columns::Columns;
use crate::wounds;

/// A unit not yet downed hasn't reckoned how long it lies (`wake_left`).
const NOT_RECKONED: f64 = -1.0;

/// Each fallen unit ([(store slot, constitution), ...], in the order it is buried) dies or is
/// downed: `None` if it dies, else the wounds it takes (1, and its last blow's). The downed's
/// hp is put to 0 and their time downed is to be reckoned.
pub fn fate(columns: &mut Columns, fallen: &[(usize, f64)]) -> Vec<Option<i64>> {
    fallen
        .iter()
        .map(|&(slot, constitution)| {
            if columns.hp[slot] as i64 <= -wounds::depth(constitution) {
                return None;
            }
            columns.hp[slot] = 0;
            columns.wake_left[slot] = NOT_RECKONED;
            Some(1 + columns.last_wounding[slot] as i64)
        })
        .collect()
}

#[cfg(test)]
mod tests;
