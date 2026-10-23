//! Climbing on stamina for a path (spec 30): where a walker can stand (ClimbStamina's
//! `standable`), what a step costs it in the climbing and swimming modes (ModeStamina's
//! `step_cost`), and a search's reckoning of it (ClimbBudget): for each cell reached, the
//! stamina spent since the last cell the walker could stand on, along the way it was
//! reached by. A step that would take a stretch past the walker's stamina is not planned;
//! reaching a standable cell ends the stretch (stamina recovers there). Each cell keeps the
//! spend of its quickest way - a slower way with less spend isn't kept - which is enough
//! where climbs are short stretches between ledges.

use crate::ground::{Ground, Mode, Walker};
use crate::maths::V2;

use super::{PathTuning, PathWalker};

const NOT_YET: u8 = 0;
const STANDS: u8 = 1;
const FACE: u8 = 2;

/// Whether `walker` can stand (and so recover stamina) on the cell holding `at`: a cell it
/// can walk (not swim) whose slope across it is no more than `cliff_quarters`.
pub fn standable(ground: &Ground, walker: &Walker, at: V2, cliff_quarters: i64) -> bool {
    if ground.crossing(walker, at, at) <= 0.0 {
        return false;
    }
    if ground.mode(walker, at, at) != Mode::Walking {
        return false;
    }
    let cell = ((at.x as f64).floor() as i32, (at.y as f64).floor() as i32);
    let steepest = slope(ground, cell, (1, 0)).max(slope(ground, cell, (0, 1)));
    steepest <= cliff_quarters
}

/// The stamina `walker`, at `speed` cells a second on open ground, spends stepping from
/// `from` into `to`: the seconds the step takes at its pace there times its mode's rate.
/// INF where it can't go.
pub fn step_cost(
    ground: &Ground,
    walker: &Walker,
    from: V2,
    to: V2,
    speed: f64,
    tuning: &PathTuning,
) -> f64 {
    let spend = match ground.mode(walker, from, to) {
        Mode::Climbing => tuning.stamina_climb,
        Mode::Swimming => tuning.stamina_swim,
        Mode::Walking => 0.0,
    };
    if spend <= 0.0 {
        return 0.0;
    }
    let pace = ground.crossing(walker, from, to) * speed;
    if pace <= 0.0 {
        f64::INFINITY
    } else {
        from.distance_to(to) as f64 / pace * spend
    }
}

/// How steadily the ground climbs through `cell` along `axis` (quarters): the lesser of its
/// rise from the neighbour behind and on to the one ahead, either way round; 0 if it
/// doesn't keep climbing.
fn slope(ground: &Ground, cell: (i32, i32), axis: (i32, i32)) -> i64 {
    let height = |x: i32, y: i32| ground.height_at(centre(x, y)) as i64;
    let here = height(cell.0, cell.1);
    let before = height(cell.0 - axis.0, cell.1 - axis.1);
    let after = height(cell.0 + axis.0, cell.1 + axis.1);
    0.max((after - here).min(here - before).max((before - here).min(here - after)))
}

fn centre(x: i32, y: i32) -> V2 {
    V2::new(x as f32 + 0.5, y as f32 + 0.5)
}

/// A search's stamina reckoning for one walker.
pub struct Budget {
    cols: i32,
    spent: Vec<f32>, // since the last standable cell, on arriving
    stands: Vec<u8>,
}

impl Budget {
    pub fn new(ground: &Ground) -> Budget {
        let (cols, rows) = ground.size();
        let count = (cols.max(0) * rows.max(0)) as usize;
        Budget { cols, spent: vec![0.0; count], stands: vec![NOT_YET; count] }
    }

    /// The stamina spent on the stretch after stepping from cell `from` into `to`
    /// (indexes), `seen` or taken as open ground; INF past the walker's stamina.
    pub fn after(
        &mut self,
        ground: &Ground,
        walker: &PathWalker,
        tuning: &PathTuning,
        (from, to): (usize, usize),
        seen: bool,
    ) -> f64 {
        let mut spent = if self.standable(ground, walker, tuning, from) {
            0.0
        } else {
            self.spent[from] as f64
        };
        if seen {
            let (a, b) = (self.centre(from), self.centre(to));
            spent += step_cost(ground, &walker.walker, a, b, walker.speed, tuning);
        }
        if spent > walker.stamina {
            f64::INFINITY
        } else {
            spent
        }
    }

    /// Keeps `spent` as the stretch's spend on reaching cell `index`.
    pub fn keep(&mut self, index: usize, spent: f64) {
        self.spent[index] = spent as f32;
    }

    fn standable(
        &mut self,
        ground: &Ground,
        walker: &PathWalker,
        tuning: &PathTuning,
        index: usize,
    ) -> bool {
        if self.stands[index] == NOT_YET {
            let stands =
                standable(ground, &walker.walker, self.centre(index), tuning.cliff_quarters);
            self.stands[index] = if stands { STANDS } else { FACE };
        }
        self.stands[index] == STANDS
    }

    fn centre(&self, index: usize) -> V2 {
        centre(index as i32 % self.cols, index as i32 / self.cols)
    }
}
