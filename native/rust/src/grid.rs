//! Finding what is near a point (as BodyGrid does in GDScript): points bucketed in square
//! cells CELL across over the box they occupy, stored flat (each cell's indices in
//! ascending order). Queries visit a superset of what qualifies; callers apply the exact
//! test and keep their own tie order, so a search through the grid picks exactly what a
//! search through the whole list would.

use crate::maths::V2;

/// Cells a bucket spans (BodyGrid.CELL): about the widest body.
pub const CELL: f32 = 2.0;
/// Cells added to a search's reach: far above float rounding, so nothing is missed.
pub const MARGIN: f64 = 0.01;

#[derive(Default)]
pub struct Grid {
    low: (i32, i32),
    width: i32,
    height: i32,
    /// Where each cell's indices start in `items` (row by row; one past the last at the end).
    start: Vec<u32>,
    items: Vec<u32>,
    /// Each point's cell, kept for the next rebuild.
    cells: Vec<(i32, i32)>,
}

#[inline]
pub fn cell_of(point: V2) -> (i32, i32) {
    ((point.x / CELL).floor() as i32, (point.y / CELL).floor() as i32)
}

/// The least distance between a point in the centre cell and one in ring `ring`
/// (BodyGrid.floor_of).
#[inline]
pub fn floor_of(ring: i32) -> f64 {
    (((ring - 1).max(0)) as f32 * CELL) as f64
}

impl Grid {
    pub fn build(points: &[V2]) -> Grid {
        let mut grid = Grid::default();
        grid.rebuild(points);
        grid
    }

    /// The grid over `points` afresh, in this grid's own buffers (spec 41 step 4): the same
    /// grid `build` makes, without allocating once the buffers are big enough.
    pub fn rebuild(&mut self, points: &[V2]) {
        (self.low, self.width, self.height) = ((0, 0), 0, 0);
        self.start.clear();
        self.items.clear();
        self.cells.clear();
        if points.is_empty() {
            return;
        }
        self.cells.extend(points.iter().map(|&p| cell_of(p)));
        let mut low = self.cells[0];
        let mut high = low;
        for &(x, y) in &self.cells {
            low = (low.0.min(x), low.1.min(y));
            high = (high.0.max(x), high.1.max(y));
        }
        let width = high.0 - low.0 + 1;
        let height = high.1 - low.1 + 1;
        let size = (width as usize) * (height as usize);
        let index = |c: (i32, i32)| ((c.1 - low.1) * width + (c.0 - low.0)) as usize;
        let start = &mut self.start;
        start.resize(size + 1, 0);
        for &c in &self.cells {
            start[index(c) + 1] += 1;
        }
        for i in 0..size {
            start[i + 1] += start[i];
        }
        // each cell's indices in ascending order, its start moved one cell on as it fills
        self.items.resize(points.len(), 0);
        for (i, &c) in self.cells.iter().enumerate() {
            let at = &mut start[index(c)];
            self.items[*at as usize] = i as u32;
            *at += 1;
        }
        start.copy_within(0..size, 1);
        start[0] = 0;
        (self.low, self.width, self.height) = (low, width, height);
    }

    #[inline]
    fn cell(&self, x: i32, y: i32) -> &[u32] {
        let i = ((y - self.low.1) * self.width + (x - self.low.0)) as usize;
        &self.items[self.start[i] as usize..self.start[i + 1] as usize]
    }

    /// Visits the indices in every cell overlapping the box round `point`, `reach` out;
    /// stops early once `visit` returns false.
    pub fn near(&self, point: V2, reach: f64, mut visit: impl FnMut(u32) -> bool) {
        if self.items.is_empty() {
            return;
        }
        let r = reach as f32;
        let from = cell_of(V2::new(point.x - r, point.y - r));
        let to = cell_of(V2::new(point.x + r, point.y + r));
        let x0 = from.0.max(self.low.0);
        let y0 = from.1.max(self.low.1);
        let x1 = to.0.min(self.low.0 + self.width - 1);
        let y1 = to.1.min(self.low.1 + self.height - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                for &i in self.cell(x, y) {
                    if !visit(i) {
                        return;
                    }
                }
            }
        }
    }

    /// The last ring round `centre` holding a cell of the grid's box (-1 if empty).
    pub fn last_ring(&self, centre: (i32, i32)) -> i32 {
        if self.items.is_empty() {
            return -1;
        }
        let high = (self.low.0 + self.width - 1, self.low.1 + self.height - 1);
        let dx = (centre.0 - self.low.0).abs().max((high.0 - centre.0).abs());
        let dy = (centre.1 - self.low.1).abs().max((high.1 - centre.1).abs());
        dx.max(dy)
    }

    /// Visits the indices in the cells exactly `ring` cells (Chebyshev) from `centre`.
    pub fn ring(&self, centre: (i32, i32), ring: i32, mut visit: impl FnMut(u32)) {
        let high = (self.low.0 + self.width - 1, self.low.1 + self.height - 1);
        let y0 = (centre.1 - ring).max(self.low.1);
        let y1 = (centre.1 + ring).min(high.1);
        let x0 = (centre.0 - ring).max(self.low.0);
        let x1 = (centre.0 + ring).min(high.0);
        for y in y0..=y1 {
            if (y - centre.1).abs() == ring {
                for x in x0..=x1 {
                    for &i in self.cell(x, y) {
                        visit(i);
                    }
                }
                continue;
            }
            for x in [centre.0 - ring, centre.0 + ring] {
                if x >= x0 && x <= x1 && (ring > 0 || x == centre.0) {
                    for &i in self.cell(x, y) {
                        visit(i);
                    }
                }
            }
        }
    }
}
