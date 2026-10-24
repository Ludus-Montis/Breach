//! Regeneration (FormationRegeneration, Decision 121; spec 39 step 6b), for recovery: a unit
//! mends HP up to its limit per rest, times its condition, through damage; a blow of a type
//! it fears stops it a while (`regeneration_halt`, which the blows' landing writes); downed,
//! it mends only with "regenerates_downed". The part of a point regained so far is the
//! store's (`regeneration_carry`).

use crate::columns::Columns;
use crate::maths::{maxf, minf};

/// What of a unit regeneration reads: its store slot, HP a second at full condition, most
/// HP, and whether it mends while downed.
#[derive(Clone, Copy, Debug)]
pub struct Mender {
    pub slot: usize,
    pub regeneration: f64,
    pub max_hp: i64,
    pub regenerates_downed: bool,
}

/// Mends what the unit may this tick of `seconds` (downed: only with "regenerates_downed").
pub fn mend(columns: &mut Columns, unit: Mender, downed: bool, seconds: f64) {
    if unit.regeneration <= 0.0 || (downed && !unit.regenerates_downed) {
        return;
    }
    let slot = unit.slot;
    if columns.regeneration_halt[slot] > 0.0 {
        columns.regeneration_halt[slot] = maxf(0.0, columns.regeneration_halt[slot] - seconds);
        return;
    }
    let mut gain = minf(
        unit.regeneration * columns.condition[slot] * seconds,
        columns.regeneration_left[slot],
    );
    let room = (unit.max_hp - columns.hp[slot] as i64) as f64;
    gain = minf(gain, room - columns.regeneration_carry[slot]);
    if gain <= 0.0 {
        return;
    }
    columns.regeneration_left[slot] -= gain;
    columns.regeneration_carry[slot] += gain;
    let whole = (columns.regeneration_carry[slot] + 0.000001).floor(); // float sums fall short
    columns.hp[slot] += whole as i32;
    columns.regeneration_carry[slot] -= whole;
}
