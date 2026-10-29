//! Morale's pressure and recovery (FormationMorale.step, Decision 82; spec 39 step 7a): once
//! a second, each formation not routing or destroyed loses morale to pressure by how many
//! sides it fights on - its front if it is locked, and each edge it is flanked on - a left or
//! right edge with a friendly formation near beyond it counting one less; or, out of contact,
//! it recovers a little by its units' mean condition (Decision 125), plus its best leader's
//! leadership; either way up to its ceiling. Its pass (`pass.rs`, spec 39 step 8b5-xxvii) reads
//! each one's morale, leadership and ceiling (`bands.rs`, over its units' courage and leaders)
//! and sets what this returns, held as `bands.rs` holds it, with its events. Nothing here reads another formation's morale, so
//! no formation's result hangs on the lists' order (Decision 97).

pub mod pass;

use crate::columns::Columns;
#[cfg(test)]
use crate::content::BattleTuning;
use crate::geometry::Shape;
use crate::maths::{bearing_vector, clampf};
use crate::roster::{Listed, Roster};
use crate::squads::Squads;

/// SquadEdges' edges, clockwise from the front.
pub const FRONT: i32 = 0;
pub const RIGHT: i32 = 1;
pub const LEFT: i32 = 3;

/// The tuning the pass reads (BattleTuning's).
#[derive(Clone, Debug)]
pub struct MoraleTuning {
    pub recovery: i64,
    pub support_cells: f64,
    pub condition_cap: f64,
    /// The pressure a second by sides fought on, the last for that many or more.
    pub pressure: Vec<i64>,
}

impl MoraleTuning {
    /// The tuning as the binding is given it: [recovery, support cells, condition cap, the
    /// pressures...].
    pub fn of_slice(t: &[f64]) -> MoraleTuning {
        MoraleTuning {
            recovery: t[0] as i64,
            support_cells: t[1],
            condition_cap: t[2],
            pressure: t[3..].iter().map(|&p| p as i64).collect(),
        }
    }

    /// The battle tuning's.
    #[cfg(test)]
    pub fn of(t: &BattleTuning) -> MoraleTuning {
        let mut all = vec![t.morale_recovery as f64, t.morale_support_cells, t.condition_cap];
        all.extend(t.morale_pressure.iter().map(|&p| p as f64));
        MoraleTuning::of_slice(&all)
    }
}

/// A formation the pass steps: its index in the roster, its morale now, its best living
/// leader's leadership and its ceiling.
#[derive(Clone, Copy, Debug)]
pub struct Stepping {
    pub listed: usize,
    pub morale: i64,
    pub leadership: i64,
    pub ceiling: i64,
}

/// Each formation's morale after this second, in the order given: before it is held to
/// 0..100 (GDScript's, with the band it crosses).
pub fn step(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    stepping: &[Stepping],
    tuning: &MoraleTuning,
) -> Vec<i64> {
    stepping
        .iter()
        .map(|s| {
            let listed = &roster.listed[s.listed];
            let sides = sides_engaged(squads, listed.squad);
            let morale = if sides.is_empty() {
                let recovered =
                    (tuning.recovery as f64 * condition(columns, listed, tuning)).round();
                s.morale + recovered as i64 + s.leadership
            } else {
                let supported = supported(squads, columns, roster, s.listed, &sides, tuning);
                let count = (sides.len() as i64 - supported).max(1) as usize;
                s.morale - tuning.pressure[count.min(tuning.pressure.len() - 1)]
            };
            morale.min(s.ceiling)
        })
        .collect()
}

/// The edges a squad is fought on: its front while locked, then its flanks, each once.
fn sides_engaged(squads: &Squads, squad: usize) -> Vec<i32> {
    let mut sides = Vec::new();
    if squads.engaged_with[squad] != 0 {
        sides.push(FRONT);
    }
    for flank in &squads.flanks[squad] {
        if !sides.contains(&flank.edge) {
            sides.push(flank.edge);
        }
    }
    sides
}

/// Its living units' mean condition, held to 0..condition_cap; 1 with none.
fn condition(columns: &Columns, listed: &Listed, tuning: &MoraleTuning) -> f64 {
    if listed.living.is_empty() {
        return 1.0;
    }
    let total = listed.living.iter().fold(0.0, |sum, m| sum + columns.condition[m.slot]);
    clampf(total / listed.living.len() as f64, 0.0, tuning.condition_cap)
}

/// How many of its fought left and right edges have a friendly formation (not destroyed)
/// near beyond them: its box, grown by support_cells, meets the friend's, and the friend's
/// centre lies out that way from its own.
fn supported(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    n: usize,
    sides: &[i32],
    tuning: &MoraleTuning,
) -> i64 {
    let me = &roster.listed[n];
    let mine = Shape::of(squads, columns, me).bounds();
    let area = mine.grow(tuning.support_cells);
    let mut covered = 0;
    for edge in [LEFT, RIGHT] {
        if !sides.contains(&edge) {
            continue;
        }
        let outward = bearing_vector(squads.heading[me.squad] + edge as f64 * 90.0);
        let near = roster.listed.iter().enumerate().any(|(k, friend)| {
            if k == n || friend.faction != me.faction || friend.living.is_empty() {
                return false;
            }
            let theirs = Shape::of(squads, columns, friend).bounds();
            let gap = theirs.center().sub(mine.center());
            area.intersects(theirs) && gap.dot(outward) > 0.0
        });
        if near {
            covered += 1;
        }
    }
    covered
}

#[cfg(test)]
mod tests;
