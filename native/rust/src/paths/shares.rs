//! Each cell's speed share for one kind of walker (WalkerShares, spec 30): the pace
//! stepping into a cell from each of its eight neighbours (`Ground::crossing`), worked out
//! the first time it is asked and kept, so a path search looks it up. Walkers of one kind -
//! the same height, abilities and modes - share one, held beside the ground and dropped
//! when it is painted (`Cache`). Off the grid a share is 0: paths keep to the grid.
//!
//! Kept in f32, as the GDScript's packed array kept them; a share worked out just now is
//! given back unrounded, as the GDScript gave it, and only a looked-up one is rounded.

use std::collections::BTreeMap;

use crate::ground::{Ground, Walker};
use crate::maths::V2;

/// The eight steps from a cell: the four sides, then the four diagonals.
pub const STEPS: [(i32, i32); 8] =
    [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1)];
/// A share not yet worked out.
const UNKNOWN: f32 = -1.0;

/// A walker's kind: its height by its bits, its swimmer and climber levels, and its modes.
type Kind = (u64, i64, i64, bool, bool);

pub struct Shares {
    walker: Walker,
    cols: i32,
    rows: i32,
    shares: Vec<f32>,
}

impl Shares {
    fn new(ground: &Ground, walker: &Walker) -> Shares {
        let (cols, rows) = ground.size();
        let count = (cols.max(0) * rows.max(0)) as usize * STEPS.len();
        Shares { walker: *walker, cols, rows, shares: vec![UNKNOWN; count] }
    }

    /// The share of its pace stepping from the cell at `index` by STEPS[`step`]: 0 off the
    /// grid or where it can't go.
    pub fn share(&mut self, ground: &Ground, index: usize, step: usize) -> f64 {
        let at = index * STEPS.len() + step;
        let mut share = self.shares[at] as f64;
        if share == UNKNOWN as f64 {
            let (x, y) = ((index as i32) % self.cols, (index as i32) / self.cols);
            let (nx, ny) = (x + STEPS[step].0, y + STEPS[step].1);
            share = 0.0;
            if nx >= 0 && ny >= 0 && nx < self.cols && ny < self.rows {
                let from = V2::new(x as f32 + 0.5, y as f32 + 0.5);
                let to = V2::new(nx as f32 + 0.5, ny as f32 + 0.5);
                share = ground.crossing(&self.walker, from, to);
            }
            self.shares[at] = share as f32;
        }
        share
    }
}

/// Each kind of walker's shares on one ground, dropped when it is painted.
#[derive(Default)]
pub struct Cache {
    kinds: BTreeMap<Kind, Shares>,
}

impl Cache {
    /// The shares for `walker`'s kind, shared by every walker of that kind.
    pub fn of(&mut self, ground: &Ground, walker: &Walker) -> &mut Shares {
        let kind =
            (walker.height.to_bits(), walker.swimmer, walker.climber, walker.swims, walker.climbs);
        self.kinds.entry(kind).or_insert_with(|| Shares::new(ground, walker))
    }

    pub fn clear(&mut self) {
        self.kinds.clear();
    }

    /// How many kinds it holds shares for.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}
