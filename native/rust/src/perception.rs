//! What each formation sees (FormationSight and FormationPerception, Decisions 87 and 134;
//! spec 39 step 4c): another squad is detected if any of its living units is within the
//! detection range of one of the formation's living units, measured in cells between unit
//! centres, and - on terrain - the sight line between them crosses at most MAX_SCREEN cells
//! that block sight (a wood: units at its edge see out and are seen, deeper in they are
//! hidden). Gathered once a tick before anything moves; a pair too far apart for either's
//! widest sight to reach is skipped before any unit is looked at.

use std::collections::HashSet;

use crate::columns::Columns;
use crate::grid::{self, Grid};
use crate::ground::Ground;
use crate::locks;
use crate::maths::V2;
use crate::rect::Rect;
use crate::roster::{Listed, Roster};
use crate::squads::Squads;

/// How many sight-blocking cells a sight line may cross and still see.
const MAX_SCREEN: usize = 2;

/// FormationSight.clear: true if the sight line between two points crosses at most
/// MAX_SCREEN blocking cells, looked at every half cell along it.
pub fn clear(ground: &Ground, from: V2, to: V2) -> bool {
    let mut screened = HashSet::new();
    let steps = ((from.distance_to(to) as f64 * 2.0).ceil() as i64).max(1);
    for index in 0..=steps {
        let at = from.lerp(to, index as f64 / steps as f64);
        if ground.blocks_sight(at) {
            screened.insert((at.x.floor() as i64, at.y.floor() as i64));
            if screened.len() > MAX_SCREEN {
                return false;
            }
        }
    }
    true
}

/// Where a squad's living units stand (their points, not their loose places).
fn points(columns: &Columns, listed: &Listed) -> Vec<V2> {
    listed.living.iter().map(|m| columns.position[m.slot]).collect()
}

/// FormationSight.detects: true if any living unit of `squad` has one of `other`'s within
/// its detection range and, on `ground`, in sight.
pub fn detects(columns: &Columns, squad: &Listed, other: &Listed, ground: Option<&Ground>) -> bool {
    let seen = points(columns, other);
    let grid = Grid::build(&seen);
    squad.living.iter().any(|unit| {
        let here = columns.position[unit.slot];
        let mut found = false;
        grid.near(here, unit.detection + grid::MARGIN, |i| {
            let there = seen[i as usize];
            if here.distance_to(there) as f64 <= unit.detection
                && ground.is_none_or(|g| clear(g, here, there))
            {
                found = true;
            }
            !found
        });
        found
    })
}

/// FormationPerception._box: the box the squad's living units' centres stand in, and its
/// widest detection (an empty box and 0 for none).
fn sight_box(columns: &Columns, listed: &Listed) -> (Rect, f64) {
    let mut low = V2::new(f32::INFINITY, f32::INFINITY);
    let mut high = V2::new(-low.x, -low.y);
    let mut widest = 0.0;
    for unit in &listed.living {
        let at = columns.position[unit.slot];
        low = V2::new(
            if low.x < at.x { low.x } else { at.x },
            if low.y < at.y { low.y } else { at.y },
        );
        high = V2::new(
            if high.x > at.x { high.x } else { at.x },
            if high.y > at.y { high.y } else { at.y },
        );
        widest = if widest > unit.detection { widest } else { unit.detection };
    }
    if widest == 0.0 && low.x == f32::INFINITY {
        return (Rect::default(), 0.0);
    }
    (Rect::new(low, high.sub(low)), widest)
}

/// Keeps what each of `roster`'s squads saw (`see`'s, in the lists' order) as whether it is
/// threatened: it saw a standing foe (Decision 134; spec 39 step 8b5-xxxvii).
pub fn keep(squads: &mut Squads, roster: &Roster, seen: &[Vec<usize>]) {
    for (listed, one) in roster.listed.iter().zip(seen) {
        squads.threatened[listed.squad] = !one.is_empty();
    }
}

/// FormationPerception.see: for each listed squad, in the lists' order, the indices of the
/// standing hostile squads it detects.
pub fn see(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    ground: Option<&Ground>,
) -> Vec<Vec<usize>> {
    let boxes: Vec<(Rect, f64)> = roster.listed.iter().map(|l| sight_box(columns, l)).collect();
    let reach: Vec<Rect> = boxes.iter().map(|(b, widest)| b.grow(widest + 1.0)).collect();
    let listed = &roster.listed;
    (0..listed.len())
        .map(|n| {
            (0..listed.len())
                .filter(|&o| {
                    let state = squads.state[listed[o].squad];
                    listed[o].faction != listed[n].faction
                        && state != locks::ROUTING
                        && state != locks::DESTROYED
                        && reach[n].intersects(boxes[o].0) // beyond its sight, or perhaps not
                        && detects(columns, &listed[n], &listed[o], ground)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
#[path = "perception/tests.rs"]
mod tests;
