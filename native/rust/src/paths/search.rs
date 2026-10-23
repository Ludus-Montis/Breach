//! One A* search over the ground's grid for a walker (PathSearch, spec 30): eight ways from
//! each cell, a diagonal only where both cells beside it can be entered (no corner cut past
//! what it can't cross); a step costs its length over the walker's pace into the cell it
//! enters (`Shares`), so it finds the quickest way, not the shortest. It keeps to the grid.
//! Its ties go to geometry (`Frontier`).
//!
//! Planning on what is seen: given a sight, the cells it sees or has seen (`Memory`, which
//! it adds to) cost what they cost; cells it can't see count as open ground (share 1). It
//! finds the quickest way to the goal on that understanding and keeps the stretch in
//! sight: the whole way if the goal is in sight and reached (`reaches`), else up to the
//! edge of sight where the way leaves it. With no sight it sees the whole grid. The goal is
//! a cell, or a route: rejoining it at the first route point reached - by walking time -
//! that lies ahead of where the group left it; with none ahead in reach, at the first
//! point behind. The estimate, the distance to the route's line, never overestimates the
//! time to a point ahead, so the first such point settled is the one Dijkstra's search
//! would find. A walker with a stamina plans only climbs whose stretches between the cells
//! it can stand on its stamina lasts (`climb::Budget`).
//!
//! PathSearch's precision: costs in f64, each cell's estimate and distance aside kept in
//! f32 (its packed arrays), points in f32.

use crate::ground::Ground;
use crate::maths::V2;
use crate::route::Route;

use super::climb::Budget;
use super::frontier::Frontier;
use super::shares::{Shares, STEPS};
use super::{centre, Cell, Eyes, PathTuning, PathWalker};

/// A diagonal step's length.
pub const DIAGONAL_STEP: f64 = std::f64::consts::SQRT_2;
/// A way must be quicker by more than this to replace one already found: float noise never
/// decides.
const BETTER: f64 = 0.000001;
/// A cell is on a route when its centre lies within this many cells of the route's line.
const ON_ROUTE: f64 = 0.5;
/// What the search knows of a cell's visibility.
const UNKNOWN: u8 = 0;
const SEEN: u8 = 1;
const UNSEEN: u8 = 2;

pub struct Search<'a> {
    /// Cells taken off the frontier and expanded, for measuring.
    pub expanded: i64,
    /// Whether the way found reaches the goal (false: it ends on the edge of sight).
    pub reaches: bool,
    ground: &'a Ground,
    shares: &'a mut Shares,
    walker: PathWalker,
    tuning: PathTuning,
    eyes: Eyes<'a>,
    cols: i32,
    rows: i32,
    count: usize,
    start: Cell,
    goal: Cell,
    route: Option<&'a Route>,
    progress: f64,         // how far along the route the group left it
    behind: Option<usize>, // the quickest-reached route cell behind that, if any
    climb: Option<Budget>, // None: a walker with no limit on its stamina
    toward: V2,            // where the straight way runs, for ties
    best: Vec<f64>,
    parent: Vec<i64>,
    closed: Vec<bool>,
    visible: Vec<u8>,
    remaining: Vec<f32>, // each cell's estimate, worked out when first asked
    asides: Vec<f32>,    // each cell's distance aside, likewise
    open: Frontier,
}

impl<'a> Search<'a> {
    pub fn new(
        ground: &'a Ground,
        shares: &'a mut Shares,
        walker: PathWalker,
        tuning: PathTuning,
        start: Cell,
        eyes: Eyes<'a>,
    ) -> Search<'a> {
        let (cols, rows) = ground.size();
        let climb = (walker.stamina < f64::INFINITY).then(|| Budget::new(ground));
        Search {
            expanded: 0,
            reaches: false,
            ground,
            shares,
            walker,
            tuning,
            eyes,
            cols,
            rows,
            count: (cols.max(0) * rows.max(0)) as usize,
            start,
            goal: (-1, -1),
            route: None,
            progress: 0.0,
            behind: None,
            climb,
            toward: V2::ZERO,
            best: Vec::new(),
            parent: Vec::new(),
            closed: Vec::new(),
            visible: Vec::new(),
            remaining: Vec::new(),
            asides: Vec::new(),
            open: Frontier::default(),
        }
    }

    /// The cells of the quickest way from start to `goal`, or to the edge of sight where
    /// the best plan runs out of it; empty if there is none.
    pub fn toward_cell(&mut self, goal: Cell) -> Vec<Cell> {
        self.goal = goal;
        self.toward = centre(goal);
        if !self.inside(goal) {
            return Vec::new();
        }
        self.run()
    }

    /// The cells of the quickest way from start onto `route` (to a cell within ON_ROUTE of
    /// its line) at or ahead of `progress` cells along it - with none in reach, behind it -
    /// or to the edge of sight where that way leaves it; empty if there is none.
    pub fn onto_route(&mut self, route: &'a Route, progress: f64) -> Vec<Cell> {
        self.route = Some(route);
        self.progress = progress;
        self.toward = route.point_at(progress.max(route.distance_of(centre(self.start))));
        self.run()
    }

    fn run(&mut self) -> Vec<Cell> {
        if !self.inside(self.start) {
            return Vec::new();
        }
        let count = self.count;
        self.best = vec![f64::INFINITY; count];
        self.parent = vec![-1; count];
        self.closed = vec![false; count];
        self.visible = vec![UNKNOWN; count];
        self.remaining = vec![-1.0; count];
        self.asides = vec![-1.0; count];
        let start = self.index(self.start);
        self.best[start] = 0.0;
        let estimate = self.estimate(start);
        let aside = self.aside(start);
        self.open.push(start, estimate, aside, estimate);
        while !self.open.is_empty() {
            let index = self.open.pop();
            if self.closed[index] {
                continue;
            }
            self.closed[index] = true;
            self.expanded += 1;
            if self.is_goal(index) {
                if self.route.is_none() || self.ahead(index) {
                    let way = self.trace(index);
                    return self.in_sight(way);
                }
                if self.behind.is_none_or(|behind| self.best[index] < self.best[behind] - BETTER) {
                    self.behind = Some(index);
                }
            }
            self.expand(index);
        }
        match self.behind {
            None => Vec::new(),
            Some(behind) => {
                let way = self.trace(behind);
                self.in_sight(way)
            }
        }
    }

    /// Offers each neighbour of the cell at `index` the way through it.
    fn expand(&mut self, index: usize) {
        let cell = self.cell(index);
        let mut sides = [0.0f32; 4];
        for (step, &(dx, dy)) in STEPS.iter().enumerate() {
            let next = (cell.0 + dx, cell.1 + dy);
            let diagonal = step >= 4;
            if diagonal {
                let across = if dx > 0 { 0 } else { 2 };
                let along = if dy > 0 { 1 } else { 3 };
                if sides[across] <= 0.0 || sides[along] <= 0.0 {
                    continue; // it would cut the corner of a cell it can't enter
                }
            }
            let mut pace = self.shares.share(self.ground, index, step); // 0 off the grid
            let inside = self.inside(next);
            let seen = !inside || self.sees(self.index(next));
            if inside && !seen {
                pace = 1.0; // what it can't see counts as open ground
            }
            if !diagonal {
                sides[step] = pace as f32;
            }
            if pace > 0.0 {
                let length = if diagonal { DIAGONAL_STEP } else { 1.0 };
                self.relax(index, next, length / pace, seen);
            }
        }
    }

    fn relax(&mut self, from: usize, cell: Cell, step: f64, seen: bool) {
        let index = self.index(cell);
        if self.closed[index] {
            return;
        }
        let through = self.best[from] + step;
        if through >= self.best[index] - BETTER {
            return;
        }
        if let Some(climb) = self.climb.as_mut() {
            let spent = climb.after(self.ground, &self.walker, &self.tuning, (from, index), seen);
            if spent == f64::INFINITY {
                return; // a climb it can't last
            }
            climb.keep(index, spent);
        }
        self.best[index] = through;
        self.parent[index] = from as i64;
        let remaining = self.estimate(index);
        let aside = self.aside(index);
        self.open.push(index, through + remaining, aside, remaining);
    }

    fn is_goal(&self, index: usize) -> bool {
        match self.route {
            None => index == self.index(self.goal),
            Some(route) => {
                let at = centre(self.cell(index));
                at.distance_to(on_route(route, at)) as f64 <= ON_ROUTE
            }
        }
    }

    fn sees(&mut self, index: usize) -> bool {
        let Some(sight) = self.eyes.sight else {
            return true;
        };
        if self.visible[index] == UNKNOWN {
            let known =
                self.eyes.memory.as_ref().is_some_and(|memory| memory.knows(index, self.count));
            let seen = known || sight.sees(Some(self.ground), centre(self.cell(index)));
            if seen {
                if let Some(memory) = self.eyes.memory.as_mut() {
                    memory.learn(index, self.count);
                }
            }
            self.visible[index] = if seen { SEEN } else { UNSEEN };
        }
        self.visible[index] == SEEN
    }

    /// The least time from the cell at `index` to the goal at full pace on open ground: the
    /// octile distance to a goal cell; for a route, the straight distance to its line less
    /// ON_ROUTE - each over the fastest ground painted, so it never overestimates.
    fn estimate(&mut self, index: usize) -> f64 {
        if self.remaining[index] < 0.0 {
            let cell = self.cell(index);
            let raw = match self.route {
                None => {
                    let (ax, ay) = ((cell.0 - self.goal.0).abs(), (cell.1 - self.goal.1).abs());
                    ax.max(ay) as f64 + (DIAGONAL_STEP - 1.0) * ax.min(ay) as f64
                }
                Some(route) => {
                    let at = centre(cell);
                    0.0f64.max(at.distance_to(on_route(route, at)) as f64 - ON_ROUTE)
                }
            };
            let kept = raw as f32;
            self.remaining[index] = (kept as f64 / self.ground.fastest()) as f32;
        }
        self.remaining[index] as f64
    }

    /// How far the cell's centre lies from the straight way from start towards the goal.
    fn aside(&mut self, index: usize) -> f64 {
        if self.asides[index] < 0.0 {
            let at = centre(self.cell(index));
            let nearest = closest_on_segment(at, centre(self.start), self.toward);
            self.asides[index] = at.distance_to(nearest);
        }
        self.asides[index] as f64
    }

    /// Whether the route cell at `index` lies at or ahead of where the group left the route
    /// (the cell holding that point counts).
    fn ahead(&self, index: usize) -> bool {
        let route = self.route.expect("a route search");
        route.distance_of(centre(self.cell(index))) + ON_ROUTE >= self.progress
    }

    /// The way as far as it stays in sight: the whole way if it does (`reaches`), else up
    /// to the last cell it sees before the way leaves its sight.
    fn in_sight(&mut self, mut way: Vec<Cell>) -> Vec<Cell> {
        for at in 0..way.len() {
            if !self.sees(self.index(way[at])) {
                self.reaches = false;
                way.truncate(at.max(1));
                return way;
            }
        }
        self.reaches = true;
        way
    }

    fn trace(&self, goal: usize) -> Vec<Cell> {
        let mut out = Vec::new();
        let mut index = goal as i64;
        while index >= 0 {
            out.push(self.cell(index as usize));
            index = self.parent[index as usize];
        }
        out.reverse();
        out
    }

    fn inside(&self, cell: Cell) -> bool {
        cell.0 >= 0 && cell.1 >= 0 && cell.0 < self.cols && cell.1 < self.rows
    }

    fn cell(&self, index: usize) -> Cell {
        (index as i32 % self.cols, index as i32 / self.cols)
    }

    fn index(&self, cell: Cell) -> usize {
        (cell.1 * self.cols + cell.0) as usize
    }
}

fn on_route(route: &Route, point: V2) -> V2 {
    route.point_at(route.distance_of(point))
}

/// Godot's `Geometry2D.get_closest_point_to_segment`, in real_t.
fn closest_on_segment(point: V2, a: V2, b: V2) -> V2 {
    let p = point.sub(a);
    let n = b.sub(a);
    let l2 = n.x * n.x + n.y * n.y;
    if l2 < 1e-20 {
        return a;
    }
    let d = n.dot(p) / l2;
    if d <= 0.0 {
        a
    } else if d >= 1.0 {
        b
    } else {
        V2::new(a.x + n.x * d, a.y + n.y * d)
    }
}
