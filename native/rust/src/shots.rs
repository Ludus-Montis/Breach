//! Ranged strikes (FormationCombat.ranged_blows, Decisions 46 and 47; spec 39 step 8a1): a
//! ranged unit strikes with its best ranged weapon from anywhere in its squad, moving or
//! fighting, at the nearest enemy unit within its range in ranks. A rank's gap from a
//! shooter goes by how far it stands ahead of the shooter's squad's front, along its heading,
//! less how far the shooter's own rank stands; ties go to a unit the shooter overlaps across
//! its squad's columns, then to the targets' seeded draws, then to the order they are listed
//! in (Decision 97). It takes that one as its target, its cooldown runs down, and at 0 it
//! shoots and starts again. Squads that have arrived neither shoot nor are shot at.
//!
//! Who may shoot is decided here too (`shooters`, spec 39 step 8a2-v): its squads standing
//! and not arrived, their living with a range, but a fighting squad's front rank, which
//! fights as melee instead. GDScript hands in each living unit's ranged time and makes the
//! shots (their damage) from what this returns.

use crate::columns::Columns;
use crate::contest;
use crate::geometry::{self, lateral_axis, Frame, Shape, CELLS, RANK_DEPTH};
use crate::locks;
use crate::maths::{bearing_vector, minf, snapped};
use crate::roster::{Member, Roster};
use crate::squads::Squads;

const EPSILON: f64 = 0.000001;

/// A unit that may shoot: its squad's index in the roster, its index in that one's living,
/// and the cooldown it starts again at once it shoots.
#[derive(Clone, Copy, Debug)]
pub struct Shooter {
    pub listed: usize,
    pub member: usize,
    pub reset: i32,
}

/// A squad's living units by rank, the ranks in the order their first unit is listed:
/// [(rank, [(index in its living, its place in the listing of every squad shot at)])].
type Ranks = Vec<(i32, Vec<(usize, usize)>)>;

/// A target's place in the shooter's choice, least first: 0 if the shooter overlaps it
/// across its columns else 1, its draw, its place in the listing.
type Choice = (u8, i64, usize);

/// Who may shoot this tick (FormationCombat.ranged_blows' choice), in the lists' order: the
/// living with a range of every squad standing and not arrived, but for a fighting squad's
/// front rank, which fights as melee. Each starts again, once it shoots, at its ranged time
/// (`seconds`, each living unit's in the roster's order) in ticks of `interval`, rounded,
/// at least 1.
pub fn shooters(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    seconds: &[f64],
    interval: i64,
) -> Vec<Shooter> {
    let mut out = Vec::new();
    let mut at = 0; // each living unit's place in `seconds`
    for (n, listed) in roster.listed.iter().enumerate() {
        let state = squads.state[listed.squad];
        for (member, unit) in listed.living.iter().enumerate() {
            let ranged = seconds[at];
            at += 1;
            let melee = state == locks::FIGHTING && columns.rank[unit.slot] == 0;
            if state == locks::ARRIVED || unit.attack_range <= 0 || melee {
                continue;
            }
            let reset = (interval as f64 * ranged).round().max(1.0) as i32;
            out.push(Shooter { listed: n, member, reset });
        }
    }
    out
}

/// This tick's shots, in the shooters' order: [(the shooter's index in `shooters`, its
/// target's squad's index in the roster, its index in that one's living)]. Sets each
/// shooter that finds a target its target, and runs its cooldown down.
pub fn shots(
    squads: &Squads,
    columns: &mut Columns,
    roster: &Roster,
    shooters: &[Shooter],
    seed: i64,
) -> Vec<(usize, usize, usize)> {
    let marks = marks(squads, columns, roster);
    let mut picks = Vec::new();
    for (s, shooter) in shooters.iter().enumerate() {
        if let Some(target) = pick(squads, columns, roster, &marks, shooter, seed) {
            picks.push((s, target));
        }
    }
    let mut out = Vec::new();
    for (s, (n, u)) in picks {
        let shooter = &shooters[s];
        let slot = roster.listed[shooter.listed].living[shooter.member].slot;
        columns.target_id[slot] = roster.listed[n].living[u].id;
        columns.attack_cooldown[slot] -= 1;
        if columns.attack_cooldown[slot] <= 0 {
            columns.attack_cooldown[slot] = shooter.reset;
            out.push((s, n, u));
        }
    }
    out
}

/// The squads a shot may strike, as [(index in the roster, its ranks)], in the lists' order.
fn marks(squads: &Squads, columns: &Columns, roster: &Roster) -> Vec<(usize, Ranks)> {
    let mut out = Vec::new();
    let mut place = 0;
    for (n, listed) in roster.listed.iter().enumerate() {
        if squads.state[listed.squad] == locks::ARRIVED {
            continue;
        }
        let mut ranks: Ranks = Vec::new();
        for (u, m) in listed.living.iter().enumerate() {
            let rank = columns.rank[m.slot];
            match ranks.iter_mut().find(|(r, _)| *r == rank) {
                Some((_, row)) => row.push((u, place)),
                None => ranks.push((rank, vec![(u, place)])),
            }
            place += 1;
        }
        if !ranks.is_empty() {
            out.push((n, ranks));
        }
    }
    out
}

/// How far a unit in `rank` of the squad framed `to` stands ahead of the front of the squad
/// framed `from`, along its heading, in tiles (SquadGeometry.unit_gap).
fn unit_gap(from: &Frame, to: &Frame, rank: i32) -> f64 {
    let point = to.anchor.sub(bearing_vector(to.heading).scale(rank as f64));
    point.sub(from.anchor).dot(bearing_vector(from.heading)) as f64 / CELLS
}

/// The nearest enemy unit within the shooter's range, as (its squad's index in the roster,
/// its index in that one's living).
fn pick(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    marks: &[(usize, Ranks)],
    shooter: &Shooter,
    seed: i64,
) -> Option<(usize, usize)> {
    let own = &roster.listed[shooter.listed];
    let unit: &Member = &own.living[shooter.member];
    let mine = Shape::of(squads, columns, own);
    let hostile: Vec<(usize, Shape, &Ranks, Vec<f64>)> = marks
        .iter()
        .filter(|(n, _)| roster.listed[*n].faction != own.faction)
        .map(|(n, ranks)| {
            let theirs = Shape::of(squads, columns, &roster.listed[*n]);
            let gaps = ranks.iter().map(|(rank, _)| unit_gap(&mine.frame, &theirs.frame, *rank));
            let gaps = gaps.collect();
            (*n, theirs, ranks, gaps)
        })
        .collect();
    let reach = unit.attack_range as f64 * RANK_DEPTH + EPSILON;
    let here = unit_gap(&mine.frame, &mine.frame, columns.rank[unit.slot]);
    let mut nearest = f64::INFINITY;
    for (_, _, _, gaps) in &hostile {
        for gap in gaps {
            let gap = (gap - here).abs();
            if gap <= reach {
                nearest = minf(nearest, snapped(gap, EPSILON));
            }
        }
    }
    if nearest == f64::INFINITY {
        return None;
    }
    let axis = lateral_axis(mine.frame.heading);
    let span = geometry::extent(&mine.corners(unit), axis);
    let mut best: Option<(Choice, (usize, usize))> = None;
    for &(n, ref theirs, ranks, ref gaps) in &hostile {
        for ((_, row), gap) in ranks.iter().zip(gaps) {
            let gap = (gap - here).abs();
            if gap > reach || snapped(gap, EPSILON) != nearest {
                continue;
            }
            for &(u, place) in row {
                let target = &theirs.living[u];
                let target_span = geometry::extent(&theirs.corners(target), axis);
                let overlap = geometry::overlap(span, target_span);
                let key =
                    (if overlap > EPSILON { 0 } else { 1 }, contest::draw(seed, target.id), place);
                if best.is_none_or(|(least, _)| key < least) {
                    best = Some((key, (n, u)));
                }
            }
        }
    }
    best.map(|(_, target)| target)
}

#[cfg(test)]
mod tests;
