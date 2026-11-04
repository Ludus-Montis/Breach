//! A map's ground (spec 39 step 8c-v): its size and the areas painted on it, in order, made
//! into a `Ground` on the battle's tuning as FormationTerrain makes and paints one. `field`
//! is the 2D feel test's field (FormationField._ground), the ground the digest's field logs
//! and BattleTrials fight on.

#[cfg(test)]
mod tests;

use super::{Ground, GroundTuning, Paint};
use crate::content::BattleTuning;

/// An area of cells, (x, y, width, height), and what is painted on it.
pub type Area = ((i32, i32, i32, i32), Paint);

pub struct Map {
    /// Columns and rows of cells.
    pub size: (i32, i32),
    /// Painted in order: a later area paints over an earlier one.
    pub areas: Vec<Area>,
}

impl Map {
    /// Open, level ground of the map's size, its areas painted in order.
    pub fn ground(&self, tuning: GroundTuning) -> Ground {
        let mut ground = Ground::new(self.size.0, self.size.1, tuning);
        for (area, paint) in &self.areas {
            ground.paint(*area, paint);
        }
        ground
    }
}

/// The map named `name` ("field": FormationField's), else None.
pub fn of(name: &str, tuning: &BattleTuning) -> Option<Map> {
    match name {
        "field" => Some(field(tuning)),
        _ => None,
    }
}

/// FormationField's ground (Decision 85), 2 tiles by 1: a wood (half pace, blocks sight,
/// the tuning's wood obscurance), a stream too deep for grems with a 4-cell ford where
/// route B crosses it, and the hill under the kingdom's reserve, rising a quarter-cell a
/// cell to 2 cells high (one ring a quarter, each ring a cell inside the last).
pub fn field(tuning: &BattleTuning) -> Map {
    let wood = Paint {
        cost: Some(0.5),
        blocks_sight: Some(true),
        obscurance: Some(tuning.sight_wood_obscurance),
        ..Paint::default()
    };
    let deep = |depth: f64| Paint { depth: Some(depth), ..Paint::default() };
    let mut areas =
        vec![((16, 0, 44, 22), wood), ((28, 0, 3, 28), deep(1.2)), ((28, 19, 3, 4), deep(0.4))];
    for ring in 0..8 {
        let hill = (92 + ring, 16 + ring, 20 - 2 * ring, 32 - 2 * ring);
        areas.push((hill, Paint { height: Some(ring + 1), ..Paint::default() }));
    }
    Map { size: (128, 64), areas }
}
