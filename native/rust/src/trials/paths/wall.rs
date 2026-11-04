//! SightTrials played in the crate (spec 39 step 8c-xvi): a grem feels along a wall (spec 30).
//! No leash: it plans on what it sees. A wall stands across its way, its only gap far to the
//! south, out of sight. It plans to the edge of its sight the best plan runs out at, walks
//! there facing the way it went, remembers what it saw (`paths::memory`) and plans again -
//! until the gap comes into view and the goal is in reach. Its sight is a short-sighted
//! grem's: 24 cells ahead, 12 to the sides, 6 behind. Printed as SightTrials.report prints it.

use std::time::Instant;

use super::{drawing, path_tuning, Area};
use crate::content::Content;
use crate::ground::{Ground, GroundTuning, Paint, Walker};
use crate::maths::V2;
use crate::paths::memory::Memory;
use crate::paths::shares::Cache;
use crate::paths::sight::Sight;
use crate::paths::{self, Cell, Eyes, Over, PathWalker};

const SIZE: (i32, i32) = (64, 40);
/// The wall, too high to climb (the gap south of it is open ground).
pub const WALL: Area = (30, 0, 2, 34);
const START: (f32, f32) = (5.5, 8.5);
const GOAL: (f32, f32) = (58.5, 8.5);
const AHEAD: f64 = 24.0;
const SIDE: f64 = 12.0;
const BEHIND: f64 = 6.0;
/// Plans before it gives up.
const MOST_PLANS: i64 = 20;

/// How it went: how many plans it made, whether the last reached the goal, every cell it
/// walked, where it planned from and the mean time a plan took (microseconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Walk {
    pub plans: i64,
    pub reaches: bool,
    pub cells: Vec<Cell>,
    pub stops: Vec<V2>,
    pub usec: i64,
}

/// SightTrials.ground: bare ground with the wall.
pub fn ground(content: &Content) -> Ground {
    let mut ground = Ground::new(SIZE.0, SIZE.1, GroundTuning::of(&content.tuning));
    ground.paint(WALL, &Paint { height: Some(8), climb: Some(3), ..Paint::default() });
    ground
}

/// SightTrials.feel_along.
pub fn feel_along(content: &Content) -> Walk {
    let ground = ground(content);
    let (tuning, mut cache) = (path_tuning(content), Cache::default());
    let grem = Walker { height: 1.0, swimmer: 0, climber: 0, swims: true, climbs: true };
    let walker = PathWalker::of(grem);
    let mut memory = Memory { known: Vec::new() };
    let (mut at, mut way, goal) =
        (V2::new(START.0, START.1), V2::new(1.0, 0.0), V2::new(GOAL.0, GOAL.1));
    let mut out = Walk { plans: 0, reaches: false, cells: Vec::new(), stops: Vec::new(), usec: 0 };
    let mut spent = 0;
    while out.plans < MOST_PLANS && !out.reaches {
        let began = Instant::now();
        let sight = sight(at, way);
        let mut over = Over { ground: &ground, cache: &mut cache, tuning };
        let eyes = Eyes { sight: Some(&sight), memory: Some(&mut memory) };
        let plan = paths::plan(&mut over, &walker, at, goal, eyes);
        spent += began.elapsed().as_micros() as i64;
        out.plans += 1;
        out.stops.push(at);
        out.cells.extend(&plan.cells);
        out.reaches = plan.reaches;
        let points = &plan.waypoints;
        if points.len() < 2 {
            break;
        }
        way = points[points.len() - 1].sub(points[points.len() - 2]).normalized();
        at = points[points.len() - 1];
    }
    out.usec = spent / out.plans.max(1);
    out
}

/// SightTrials.report: what it did, and its walk drawn over the ground.
pub fn report(walk: &Walk, content: &Content) -> Vec<String> {
    let stops: Vec<String> =
        walk.stops.iter().map(|at| format!("({}, {})", at.x as i64, at.y as i64)).collect();
    let how = if walk.reaches { "reached the goal" } else { "gave up" };
    let (ahead, side, behind) = (AHEAD as i64, SIDE as i64, BEHIND as i64);
    let mut out = vec![
        format!("\nfeeling along a wall (sight {ahead} ahead, {side} aside, {behind} behind):"),
        format!(
            "  {} plans, {how}, {} us a plan; planned from {}",
            walk.plans,
            walk.usec,
            stops.join(", ")
        ),
    ];
    let view = (0, 0, SIZE.0, SIZE.1);
    out.extend(drawing(&walk.cells, view, 1, &ground(content), &[WALL]));
    out
}

/// PathSight.new(at, way, AHEAD, SIDE, BEHIND): facing `way` normalized, over no ground.
fn sight(at: V2, way: V2) -> Sight {
    let facing = way.normalized();
    Sight {
        origin: at,
        facing,
        ahead: AHEAD,
        side: SIDE,
        behind: BEHIND,
        hides: false,
        budget: f64::INFINITY,
    }
}
