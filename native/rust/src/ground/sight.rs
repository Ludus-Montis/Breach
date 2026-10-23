//! How much each cell of the ground hides what lies beyond it (spec 30; TerrainObscurance
//! until spec 39 step 3a): one rule for sight. Each cell has an obscurance - what a cell of
//! distance through it costs a sight line - and `fog`, the field's weather, adds its own to
//! every cell, on the grid or off it. A sight line sums the obscurance cell by cell, each
//! cell's by the distance the line runs through it, and sight ends where the sum passes the
//! looker's sight budget (BattleTuning.sight_budget). A wood (sight_wood_obscurance) is
//! seen a few cells into, then nothing; fog (sight_fog_obscurance, painted or as weather)
//! shortens sight rather than blocking it. The sum is the same either way along a line and
//! mirrored with the ground: a line running along a cell edge takes the mean of the cells
//! either side, and a corner it only touches costs nothing.
//!
//! PathSight plans by it. FormationSight's detection (a line crossing more than MAX_SCREEN
//! cells that block sight is blocked) will move to it too, so detection and planning see
//! alike. The maths is TerrainObscurance's: cells in f32, points and the box in f32
//! (`Vector2`, `Rect2`), the sums in f64.

use serde::{Deserialize, Serialize};

use crate::maths::V2;

/// A sum within this of the budget still sees: float noise never decides.
const SLACK: f64 = 0.000001;

/// Godot's `Rect2` (real_t = f32): a position and a size.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    /// `Rect2::merge`.
    fn merge(self, o: Rect) -> Rect {
        let (x, y) = (min(o.x, self.x), min(o.y, self.y));
        let (ex, ey) = (max(o.x + o.w, self.x + self.w), max(o.y + o.h, self.y + self.h));
        Rect { x, y, w: ex - x, h: ey - y }
    }

    /// `Rect2(at, Vector2.ZERO).expand(to)`.
    fn spanning(at: V2, to: V2) -> Rect {
        let (mut bx, mut by, mut ex, mut ey) = (at.x, at.y, at.x, at.y);
        if to.x < bx {
            bx = to.x;
        }
        if to.y < by {
            by = to.y;
        }
        if to.x > ex {
            ex = to.x;
        }
        if to.y > ey {
            ey = to.y;
        }
        Rect { x: bx, y: by, w: ex - bx, h: ey - by }
    }

    /// `Rect2::intersects(o, true)`: borders count.
    fn touches(self, o: Rect) -> bool {
        !(self.x > o.x + o.w
            || self.x + self.w < o.x
            || self.y > o.y + o.h
            || self.y + self.h < o.y)
    }
}

/// Godot's MIN and MAX.
fn min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Obscurance {
    /// The weather's obscurance, added to every cell's.
    pub fog: f64,
    cols: i32,
    rows: i32,
    cells: Vec<f32>,
    /// The cells with an obscurance of their own all lie within it.
    bounds: Rect,
    /// Whether any does.
    any: bool,
}

impl Obscurance {
    pub fn new(cols: i32, rows: i32) -> Obscurance {
        let count = (cols.max(0) * rows.max(0)) as usize;
        Obscurance {
            fog: 0.0,
            cols,
            rows,
            cells: vec![0.0; count],
            bounds: Rect::default(),
            any: false,
        }
    }

    /// The obscurance of the cell holding `point` (the mean of the cells either side where
    /// it lies on a cell edge), with the fog.
    pub fn at(&self, point: V2) -> f64 {
        let (px, py) = (point.x as f64, point.y as f64);
        let cell = (px.floor() as i64, py.floor() as i64);
        if cell.0 as f64 != px && cell.1 as f64 != py {
            return self.own(cell) + self.fog;
        }
        let (xs, ys) = (sides(px), sides(py));
        let mut total = 0.0;
        for &x in &xs {
            for &y in &ys {
                total += self.own((x, y));
            }
        }
        total / (xs.len() * ys.len()) as f64 + self.fog
    }

    /// The obscurance summed along the sight line from `from` to `to`: each cell's times the
    /// distance the line runs through it, cell by cell. It stops counting once past `limit`.
    pub fn along(&self, from: V2, to: V2, limit: f64) -> f64 {
        let length = from.distance_to(to) as f64;
        if length <= 0.0 {
            return 0.0;
        }
        if !self.any || !Rect::spanning(from, to).touches(self.bounds) {
            return self.fog * length;
        }
        let (fx, fy, tx, ty) = (from.x as f64, from.y as f64, to.x as f64, to.y as f64);
        let mut x_edge = first_edge(fx, tx);
        let mut y_edge = first_edge(fy, ty);
        let mut total = 0.0;
        let mut done = 0.0; // how far along the line (0 to 1) it has summed
        while done < 1.0 {
            let x_cut = share(fx, tx, x_edge);
            let y_cut = share(fy, ty, y_edge);
            let cut = x_cut.min(y_cut).min(1.0);
            if cut > done {
                total += self.at(lerp(from, to, (cut + done) * 0.5)) * (cut - done) * length;
                if total > limit {
                    return total;
                }
            }
            if cut == x_cut {
                x_edge += if to.x > from.x { 1 } else { -1 };
            }
            if cut == y_cut {
                y_edge += if to.y > from.y { 1 } else { -1 };
            }
            done = cut;
        }
        total
    }

    /// Whether a looker with sight budget `budget` at `from` sees `to`.
    pub fn clear(&self, from: V2, to: V2, budget: f64) -> bool {
        self.along(from, to, budget + SLACK) <= budget + SLACK
    }

    /// Sets the obscurance of the cell at `index` on the grid.
    pub fn set_cell(&mut self, index: usize, value: f64) {
        self.cells[index] = value as f32;
        if value == 0.0 {
            return;
        }
        let cols = self.cols as usize;
        let cell = Rect { x: (index % cols) as f32, y: (index / cols) as f32, w: 1.0, h: 1.0 };
        self.bounds = if self.any { self.bounds.merge(cell) } else { cell };
        self.any = true;
    }

    fn own(&self, (x, y): (i64, i64)) -> f64 {
        if x < 0 || y < 0 || x >= self.cols as i64 || y >= self.rows as i64 {
            return 0.0;
        }
        self.cells[(y * self.cols as i64 + x) as usize] as f64
    }
}

/// `Vector2::lerp`: the weight narrowed to real_t, `from + (to - from) * weight` per axis.
fn lerp(from: V2, to: V2, weight: f64) -> V2 {
    let w = weight as f32;
    V2::new(from.x + (to.x - from.x) * w, from.y + (to.y - from.y) * w)
}

/// The cells (along one axis) a coordinate lies in: both where it is on an edge.
fn sides(coordinate: f64) -> Vec<i64> {
    let cell = coordinate.floor() as i64;
    if cell as f64 == coordinate {
        vec![cell - 1, cell]
    } else {
        vec![cell]
    }
}

/// The first cell edge (on one axis) a line from `start` to `end` crosses after leaving.
fn first_edge(start: f64, end: f64) -> i64 {
    if end > start {
        start.floor() as i64 + 1
    } else {
        start.ceil() as i64 - 1
    }
}

/// Where (0 to 1 along the line) a line from `start` to `end` on one axis crosses `edge`;
/// infinity if it never does.
fn share(start: f64, end: f64, edge: i64) -> f64 {
    if start == end {
        return f64::INFINITY;
    }
    let share = (edge as f64 - start) / (end - start);
    if share > 0.0 {
        share
    } else {
        f64::INFINITY
    }
}
