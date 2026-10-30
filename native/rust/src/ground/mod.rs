//! The ground a formation fight is on (Decision 85, spec 27 round 4; FormationTerrain until
//! spec 39 step 3a): a grid of cells, each with a move cost, a height (in quarters of a
//! cell), a liquid depth (in cells), how fast its liquid flows, the climb difficulty of its
//! faces, whether it blocks sight and its obscurance (`sight`). Outside the grid is open,
//! level ground. A walker's pace into a cell (`crossing`) is the product of:
//! - **ground:** the cell's move cost (a wood 0.5)
//! - **slope:** each quarter-cell risen costs a share of the pace; downhill is no faster; a
//!   rise past the cliff height is a cliff, climbed
//! - **liquid,** in bands of the walker's own height: under a quarter free, to a half
//!   wading, to its height slow wading, deeper swum
//!
//! Climbing and swimming are movement modes (TerrainWalker): the mode's base pace times
//! Decision 64's pair rule, the walker's climber or swimmer level against the cell's climb
//! difficulty or flow (spec 30). The shares and heights are BattleTuning's (`ground_*`).
//!
//! The maths is FormationTerrain's, bit for bit: cells held in its precision (cost, depth
//! and obscurance in f32, as its packed arrays held them), points in f32 (`Vector2`), and
//! the shares in f64 (GDScript floats). The map's surface, before it is a grid, is
//! `surface` (GroundSurface).

pub mod map;
pub mod sight;
pub mod surface;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::content::BattleTuning;
use crate::maths::V2;

/// The ground's shares and heights (BattleTuning's `ground_*`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GroundTuning {
    pub cliff_quarters: i64,
    pub slope_cost: f64,
    pub wading: f64,
    pub slow_wading: f64,
    pub swim_pace: f64,
    pub climb_pace: f64,
    pub climb_demand: i64,
}

impl GroundTuning {
    pub fn of(tuning: &BattleTuning) -> GroundTuning {
        GroundTuning {
            cliff_quarters: tuning.ground_cliff_quarters,
            slope_cost: tuning.ground_slope_cost,
            wading: tuning.ground_wading,
            slow_wading: tuning.ground_slow_wading,
            swim_pace: tuning.ground_swim_pace,
            climb_pace: tuning.ground_climb_pace,
            climb_demand: tuning.ground_climb_demand,
        }
    }
}

/// What of a unit its crossing reads (TerrainWalker): its height in cells, its swimmer and
/// climber levels, and whether it may swim and climb at all.
#[derive(Clone, Copy, Debug)]
pub struct Walker {
    pub height: f64,
    pub swimmer: i64,
    pub climber: i64,
    pub swims: bool,
    pub climbs: bool,
}

/// TerrainWalker.Mode, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Walking = 0,
    Climbing = 1,
    Swimming = 2,
}

/// What a paint sets on its cells; `None` leaves a cell's own.
#[derive(Clone, Debug, Default)]
pub struct Paint {
    pub cost: Option<f64>,
    pub height: Option<i32>,
    pub depth: Option<f64>,
    pub climb: Option<i32>,
    pub flows: Option<i32>,
    pub obscurance: Option<f64>,
    pub blocks_sight: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ground {
    cols: i32,
    rows: i32,
    tuning: GroundTuning,
    /// The fastest ground painted (a road's cost above 1): searches divide their estimates
    /// by it, so a quicker way along a road is never overlooked (PathSearch).
    fastest: f64,
    cost: Vec<f32>,
    height: Vec<i32>,
    depth: Vec<f32>,
    blocks: Vec<u8>,
    climb: Vec<i32>,
    flows: Vec<i32>,
    sight: sight::Obscurance,
}

impl Ground {
    /// Open, level ground `cols` by `rows`, each cell's faces at the plain climb demand.
    pub fn new(cols: i32, rows: i32, tuning: GroundTuning) -> Ground {
        let count = (cols.max(0) * rows.max(0)) as usize;
        Ground {
            cols,
            rows,
            tuning,
            fastest: 1.0,
            cost: vec![1.0; count],
            height: vec![0; count],
            depth: vec![0.0; count],
            blocks: vec![0; count],
            climb: vec![tuning.climb_demand as i32; count],
            flows: vec![0; count],
            sight: sight::Obscurance::new(cols, rows),
        }
    }

    pub fn fastest(&self) -> f64 {
        self.fastest
    }

    /// Its columns and rows.
    pub fn size(&self) -> (i32, i32) {
        (self.cols, self.rows)
    }

    /// Sets the cells of `area` (x, y, width, height) on the grid to what `paint` gives.
    pub fn paint(&mut self, area: (i32, i32, i32, i32), paint: &Paint) {
        let (x, y, w, h) = area;
        for row in y.max(0)..(y + h).min(self.rows) {
            for col in x.max(0)..(x + w).min(self.cols) {
                let index = (row * self.cols + col) as usize;
                if let Some(cost) = paint.cost {
                    self.cost[index] = cost as f32;
                }
                self.fastest = self.fastest.max(self.cost[index] as f64);
                if let Some(height) = paint.height {
                    self.height[index] = height;
                }
                if let Some(depth) = paint.depth {
                    self.depth[index] = depth as f32;
                }
                if let Some(climb) = paint.climb {
                    self.climb[index] = climb;
                }
                if let Some(flows) = paint.flows {
                    self.flows[index] = flows;
                }
                if let Some(obscurance) = paint.obscurance {
                    self.sight.set_cell(index, obscurance);
                }
                if let Some(blocks) = paint.blocks_sight {
                    self.blocks[index] = blocks as u8;
                }
            }
        }
    }

    /// The height (quarters) of the cell holding a point.
    pub fn height_at(&self, at: V2) -> i32 {
        self.index(at).map_or(0, |index| self.height[index])
    }

    pub fn blocks_sight(&self, at: V2) -> bool {
        self.index(at).is_some_and(|index| self.blocks[index] == 1)
    }

    /// True if `from` stands at least a quarter-cell higher than `to`: a striker there has
    /// the high ground (Decision 85).
    pub fn high_ground(&self, from: V2, to: V2) -> bool {
        self.height_at(from) > self.height_at(to)
    }

    /// How fast `walker` goes stepping from `from` into `to`, as a share of its speed on
    /// open level ground; 0 where it can't go.
    pub fn crossing(&self, walker: &Walker, from: V2, to: V2) -> f64 {
        let Some(index) = self.index(to) else {
            return 1.0;
        };
        let t = &self.tuning;
        let rise = self.height[index] as i64 - self.height_at(from) as i64;
        let share = self.cost[index] as f64 * self.rise_share(walker, rise, index);
        let depth = self.depth[index] as f64 / walker.height.max(0.01);
        if depth >= 1.0 {
            return share
                * mode_share(walker.swims, t.swim_pace, walker.swimmer, self.flows[index] as i64);
        }
        if depth >= 0.5 {
            return share * t.slow_wading;
        }
        if depth >= 0.25 {
            return share * t.wading;
        }
        share
    }

    /// The movement mode of `walker` stepping from `from` into `to`: climbing a cliff, else
    /// swimming water at least its height deep, else walking (spec 30).
    pub fn mode(&self, walker: &Walker, from: V2, to: V2) -> Mode {
        let Some(index) = self.index(to) else {
            return Mode::Walking;
        };
        if self.height[index] as i64 - self.height_at(from) as i64 > self.tuning.cliff_quarters {
            return Mode::Climbing;
        }
        if self.depth[index] as f64 >= walker.height {
            return Mode::Swimming;
        }
        Mode::Walking
    }

    /// The obscurance of the cell holding `point`, with the fog (`sight`).
    pub fn obscurance_at(&self, point: V2) -> f64 {
        self.sight.at(point)
    }

    /// The obscurance summed along the sight line from `from` to `to` (`sight`).
    pub fn sight_along(&self, from: V2, to: V2, limit: f64) -> f64 {
        self.sight.along(from, to, limit)
    }

    /// Whether a looker with sight budget `budget` at `from` sees `to`.
    pub fn sight_clear(&self, from: V2, to: V2, budget: f64) -> bool {
        self.sight.clear(from, to, budget)
    }

    /// The weather's obscurance, added to every cell's.
    pub fn fog(&self) -> f64 {
        self.sight.fog
    }

    pub fn set_fog(&mut self, fog: f64) {
        self.sight.fog = fog;
    }

    /// The share of its pace a rise of `rise` quarters into cell `index` leaves `walker`: a
    /// slope's toll, or a cliff's climb by the pair rule.
    fn rise_share(&self, walker: &Walker, rise: i64, index: usize) -> f64 {
        let t = &self.tuning;
        if rise > t.cliff_quarters {
            return mode_share(
                walker.climbs,
                t.climb_pace,
                walker.climber,
                self.climb[index] as i64,
            );
        }
        0.1f64.max(1.0 - t.slope_cost * rise.max(0) as f64)
    }

    fn index(&self, at: V2) -> Option<usize> {
        let (x, y) = ((at.x as f64).floor() as i64, (at.y as f64).floor() as i64);
        if x < 0 || y < 0 || x >= self.cols as i64 || y >= self.rows as i64 {
            return None;
        }
        Some((y * self.cols as i64 + x) as usize)
    }
}

/// Decision 64's pair rule (TerrainWalker.meets): full at or above the demand, half one
/// short, none beyond.
pub fn meets(ability: i64, demand: i64) -> f64 {
    if ability >= demand {
        1.0
    } else if ability == demand - 1 {
        0.5
    } else {
        0.0
    }
}

/// A movement mode's share of the pace: its base pace times the pair rule, `ability`
/// against `demand`; 0 for a walker that may not use it.
fn mode_share(allowed: bool, pace: f64, ability: i64, demand: i64) -> f64 {
    if allowed {
        pace * meets(ability, demand)
    } else {
        0.0
    }
}
