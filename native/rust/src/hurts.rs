//! Landing the fight's hurts (FormationSimulation._fight, spec 39 step 8b2): each shot, then
//! each blow, in the order landed (BlowLanding), takes what it dealt from its target's hp
//! (UnitHealth.hurt); a unit taken from above 0 to 0 or below falls, onto the fell queue in
//! that order, for deaths to take. Each hurt's answer is its target's hp after it, which the
//! "hit" event reports. The events themselves are GDScript's till the tick hands them back as
//! data (step 8b5).

#[cfg(test)]
mod tests;

use crate::columns::Columns;
use crate::wounds;

/// One hurt landed: its target's store slot and id, and the damage it dealt.
#[derive(Clone, Copy, Debug)]
pub struct Hurt {
    pub slot: usize,
    pub id: i64,
    pub damage: i64,
}

/// Lands `hurts` in order on the columns' hp, the fallen pushed onto `fell`; returns each
/// one's target's hp after it.
pub fn land(columns: &mut Columns, hurts: &[Hurt], fell: &mut Vec<i64>) -> Vec<i32> {
    hurts
        .iter()
        .map(|h| {
            wounds::hurt(columns, h.slot, h.id, h.damage, fell);
            columns.hp[h.slot]
        })
        .collect()
}
