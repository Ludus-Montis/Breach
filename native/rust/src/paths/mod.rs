//! Paths over the ground for a walker (TerrainPaths, spec 30; Decisions 75, 85 and 97; spec
//! 39 step 3c): the quickest way at the walker's own pace on each cell - the pace the march
//! uses (`Ground::crossing`, looked up through `shares`): ground, slope, liquid bands by
//! its height, swimming and climbing by its abilities. A ford beats swimming, or a wood a
//! detour, by cost alone. There is no leash: a pathfinder plans on what it sees (`sight`) -
//! seen cells at their real cost, unseen ones as open ground - and where the best plan runs
//! out of sight it heads for that edge and plans again from there (`search`), remembering
//! what it has seen (`memory`, optional) so a wall felt along stays known. Deterministic:
//! ties go to geometry (`frontier`), so the same ground gives the same path and the ground
//! mirrored gives it mirrored.
//!
//! TerrainPaths' precision: points in f32 (`Vector2`), times in f64 (GDScript floats).

pub mod climb;
mod frontier;
pub mod memory;
pub mod search;
pub mod shares;
pub mod sight;
#[cfg(test)]
mod tests;

use crate::ground::{Ground, Walker};
use crate::maths::V2;
use crate::route::Route;

use memory::Memory;
use search::Search;
use shares::Cache;
use sight::Sight;

/// How finely (cells) a straight leg is sampled for its cost.
const SAMPLE: f64 = 0.2;
/// A straight leg replaces a stretch of the way only if it is no slower than that by more
/// than this (seconds): float noise never decides.
const SLACK: f64 = 0.000001;

/// A cell of the grid: (column, row).
pub type Cell = (i32, i32);

/// What of a walker a path reads: its crossing, the stamina it has for a climb (INF: no
/// limit) and its speed on open ground (cells a second).
#[derive(Clone, Copy, Debug)]
pub struct PathWalker {
    pub walker: Walker,
    pub stamina: f64,
    pub speed: f64,
}

impl PathWalker {
    /// A walker with no limit on its stamina, at a cell a second.
    pub fn of(walker: Walker) -> PathWalker {
        PathWalker { walker, stamina: f64::INFINITY, speed: 1.0 }
    }
}

/// The tuning a climb reads (BattleTuning's ground_cliff_quarters, stamina_climb and
/// stamina_swim), as it stands when the path is planned.
#[derive(Clone, Copy, Debug)]
pub struct PathTuning {
    pub cliff_quarters: i64,
    pub stamina_climb: f64,
    pub stamina_swim: f64,
}

/// What a plan is made over: the ground, its walkers' shares and the tuning.
pub struct Over<'a> {
    pub ground: &'a Ground,
    pub cache: &'a mut Cache,
    pub tuning: PathTuning,
}

/// What a pathfinder plans by: what it sees (None: everything) and what it has seen.
#[derive(Default)]
pub struct Eyes<'a> {
    pub sight: Option<&'a Sight>,
    pub memory: Option<&'a mut Memory>,
}

/// A way planned: `waypoints` from the start, the centres of the cells where it turns, and
/// the goal - or the cell on the edge of sight it heads for (empty where there is no way);
/// whether it `reaches` the goal; the way's `cells`; for a rejoin, how far `along` the
/// route it rejoins (-1 short of it).
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub waypoints: Vec<V2>,
    pub reaches: bool,
    pub cells: Vec<Cell>,
    pub along: f64,
}

/// A search from the cell holding `from`, with the walker's shares from the cache.
pub fn search<'a>(
    over: &'a mut Over,
    walker: &PathWalker,
    from: Cell,
    eyes: Eyes<'a>,
) -> Search<'a> {
    let shares = over.cache.of(over.ground, &walker.walker);
    Search::new(over.ground, shares, *walker, over.tuning, from, eyes)
}

/// The way planned from `from` towards `goal` on what `eyes` see.
pub fn plan(over: &mut Over, walker: &PathWalker, from: V2, goal: V2, eyes: Eyes) -> Plan {
    let ground = over.ground;
    let mut search = search(over, walker, cell_of(from), eyes);
    let way = search.toward_cell(cell_of(goal));
    let reaches = search.reaches;
    let end = reaches.then_some(goal);
    planned(ground, &walker.walker, way, from, end, reaches)
}

/// The way back onto `route` from `from` on what `eyes` see: to the first cell on the
/// route reached that lies at or ahead of `progress` cells along it, where the group left
/// it (negative: where `from` lies along it) - with none in reach, the first behind - its
/// distance along the route `along`; or where that way leaves sight, to the edge of sight
/// (`reaches` false, `along` -1). As `plan` otherwise.
pub fn rejoin(
    over: &mut Over,
    walker: &PathWalker,
    from: V2,
    route: &Route,
    progress: f64,
    eyes: Eyes,
) -> Plan {
    let ground = over.ground;
    let mut search = search(over, walker, cell_of(from), eyes);
    let left = if progress < 0.0 { route.distance_of(from) } else { progress };
    let way = search.onto_route(route, left);
    let reaches = search.reaches;
    let mut out = planned(ground, &walker.walker, way, from, None, reaches);
    out.along = match out.waypoints.last() {
        Some(&last) if reaches => route.distance_of(last),
        _ => -1.0,
    };
    out
}

/// The time (seconds, at a speed of a cell a second) `walker` takes along `waypoints`,
/// sampled every SAMPLE cells; INF if a leg crosses where it can't go.
pub fn cost(ground: &Ground, walker: &Walker, waypoints: &[V2]) -> f64 {
    let mut total = 0.0;
    for leg in waypoints.windows(2) {
        total += leg_time(ground, walker, leg[0], leg[1]);
    }
    total
}

/// The plan along `way`: from `from` to `end` (None: the last cell's centre), straightened.
fn planned(
    ground: &Ground,
    walker: &Walker,
    way: Vec<Cell>,
    from: V2,
    end: Option<V2>,
    reaches: bool,
) -> Plan {
    let mut points: Vec<V2> = way.iter().map(|&cell| centre(cell)).collect();
    if !points.is_empty() {
        points[0] = from;
        if let Some(end) = end {
            if points.len() > 1 {
                let last = points.len() - 1;
                points[last] = end;
            }
        }
        points = straightened(ground, walker, &points);
    }
    Plan { waypoints: points, reaches: reaches && !way.is_empty(), cells: way, along: -1.0 }
}

/// The cell holding a point (`floori` of each coordinate).
pub fn cell_of(point: V2) -> Cell {
    ((point.x as f64).floor() as i32, (point.y as f64).floor() as i32)
}

/// A cell's centre.
pub fn centre(cell: Cell) -> V2 {
    V2::new(cell.0 as f32 + 0.5, cell.1 as f32 + 0.5)
}

/// Each waypoint kept reaches as far along the way as a straight leg can: galloping out
/// (2, 4, 8... points on), then halving back between the last leg that held and the first
/// that didn't.
fn straightened(ground: &Ground, walker: &Walker, points: &[V2]) -> Vec<V2> {
    let mut along = vec![0.0f64]; // the time to each point along the way
    for index in 1..points.len() {
        along.push(along[index - 1] + leg_time(ground, walker, points[index - 1], points[index]));
    }
    let holds = |from: usize, to: usize| {
        leg_time(ground, walker, points[from], points[to]) <= along[to] - along[from] + SLACK
    };
    let mut out = vec![points[0]];
    let mut at = 0;
    let last = points.len() - 1;
    while at < last {
        let mut held = at + 1;
        let mut reach = 2;
        while at + reach <= last && holds(at, at + reach) {
            held = at + reach;
            reach *= 2;
        }
        let mut failed = (at + reach).min(last + 1);
        while failed - held > 1 {
            let middle = (held + failed) >> 1;
            if holds(at, middle) {
                held = middle;
            } else {
                failed = middle;
            }
        }
        out.push(points[held]);
        at = held;
    }
    out
}

/// The time along the straight leg from `start` to `end`: each sample's stretch at the pace
/// into the cell it lies in from the last sample's. Samples fall mid-stretch, off the
/// grid's lines; one crossing a corner needs both cells beside it enterable.
fn leg_time(ground: &Ground, walker: &Walker, start: V2, end: V2) -> f64 {
    let length = start.distance_to(end) as f64;
    let count = 1i64.max((length / SAMPLE).ceil() as i64);
    let mut total = 0.0;
    let mut last = start;
    for step in 1..=count {
        let point =
            if step < count { start.lerp(end, (step as f64 - 0.5) / count as f64) } else { end };
        let pace = ground.crossing(walker, last, point);
        if pace <= 0.0 || !corner_clear(ground, walker, last, point) {
            return f64::INFINITY;
        }
        total += (length / count as f64) / pace;
        last = point;
    }
    total
}

/// Whether a move from `last` to `point` passing diagonally between cells could enter both
/// cells beside the corner it passes.
fn corner_clear(ground: &Ground, walker: &Walker, last: V2, point: V2) -> bool {
    let was = cell_of(last);
    let now = cell_of(point);
    if was.0 == now.0 || was.1 == now.1 {
        return true;
    }
    for cell in [(now.0, was.1), (was.0, now.1)] {
        if ground.crossing(walker, last, centre(cell)) <= 0.0 {
            return false;
        }
    }
    true
}
