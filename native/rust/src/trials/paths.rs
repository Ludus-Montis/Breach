//! PathTrials played in the crate (spec 39 step 8c-xvi), for `breach_trials scenario=paths`:
//! ways over the feel test's field (FormationField's ground: its grass, wood, stream with a
//! 4-cell ford and hill) with a crag added - a 2-cell cliff across the south, climb difficulty
//! 2, open to the north - for a climber to climb. Four walkers go each trip: a grem, a
//! militiaman loaded past the mode load, a cart that neither swims nor climbs and a grem
//! climber 2. Two trips see the whole field; three plan on what a grem sees (its sight shape,
//! facing east). Each search is timed (the mean of `repeats`) and the whole-field ways drawn
//! as text: '.' grass, 'T' wood, '=' wading water, '~' deep water, '^' hill, '#' crag, '*' the
//! way. Printed as tools/formation_trials.gd prints PathTrials.report, so the two outputs can
//! be compared line for line once the timings are left out (CI's `trials` job does).

#[cfg(test)]
mod tests;
pub mod wall;

use crate::hashing::HashSet;
use std::collections::BTreeMap;
use std::time::Instant;

use crate::arms::Arms;
use crate::content::{Armour, Content, Item, Unit};
use crate::field_play::{routes, A};
use crate::ground::{map, Ground, GroundTuning, Paint, Walker};
use crate::made;
use crate::maths::V2;
use crate::paths::shares::Cache;
use crate::paths::sight::Sight;
use crate::paths::{self, Cell, Eyes, Over, PathTuning, PathWalker, Plan};
use crate::route::Route;

/// An area of the grid: (x, y, width, height), as a Rect2i.
pub type Area = (i32, i32, i32, i32);

const GREM: &str = "grem";
const MILITIA: &str = "kingdom_militia";
/// The crag: a 2-cell cliff across the south, too steep but for a climber 2.
pub const CRAG: Area = (70, 26, 4, 31);
const CRAG_QUARTERS: i32 = 8;
const CRAG_CLIMB: i32 = 2;
/// What the militiaman carries on top of his spear: past the mode load.
const PACK_WEIGHT: f64 = 14.0;
const PACK: &str = "pack";
const FIELD_VIEW: Area = (0, 0, 128, 64);
const STREAM_VIEW: Area = (14, 0, 34, 26);

/// A trip: from a point to a point (None: back onto route A), planned on what a grem facing
/// east sees (`sight`) or on the whole field.
pub struct Trip {
    pub name: &'static str,
    from: (f32, f32),
    to: Option<(f32, f32)>,
    sight: bool,
}

/// PathTrials.TRIPS, in its order.
pub const TRIPS: [Trip; 5] = [
    Trip { name: "across", from: (2.5, 40.5), to: Some((125.5, 40.5)), sight: false },
    Trip { name: "stream", from: (20.5, 6.5), to: Some((40.5, 6.5)), sight: false },
    Trip { name: "sight 40 ahead", from: (20.5, 40.5), to: Some((125.5, 40.5)), sight: true },
    Trip { name: "sight into wood", from: (8.5, 12.5), to: Some((125.5, 12.5)), sight: true },
    Trip { name: "rejoin route A", from: (50.5, 45.5), to: None, sight: true },
];

/// PathTrials.WALKERS, in its order.
pub const WALKERS: [&str; 4] = ["grem", "loaded militiaman", "cart (sinks)", "climber"];

/// A trip and walker: the way planned, the seconds along it at the walker's speed, the cells
/// a search searched and the mean time a search took (microseconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub trip: &'static str,
    pub walker: &'static str,
    pub plan: Plan,
    pub seconds: f64,
    pub expanded: i64,
    pub usec: i64,
}

/// PathTrials.run: a row a trip and walker, each search the mean of `repeats`; None if a
/// definition it needs isn't in `content`.
pub fn run(repeats: i64, content: &Content) -> Option<Vec<Row>> {
    let content = packed(content);
    let ground = ground(&content)?;
    let (tuning, mut cache) = (path_tuning(&content), Cache::default());
    let route = routes().swap_remove(A);
    let grem = content.units.get(GREM)?;
    let mut rows = Vec::new();
    for trip in &TRIPS {
        for walker in WALKERS {
            let unit = unit(&content, walker)?;
            let path_walker = PathWalker::of(walker_of(&content, &unit));
            let from = V2::new(trip.from.0, trip.from.1);
            let sight = trip.sight.then(|| sight_of(grem, from, &content));
            let mut over = Over { ground: &ground, cache: &mut cache, tuning };
            let way = Way { walker: &path_walker, trip, route: &route, sight: sight.as_ref() };
            let (expanded, usec) = way.timed(&mut over, repeats);
            let plan = way.planned(&mut over);
            let seconds = paths::cost(&ground, &path_walker.walker, &plan.waypoints) / unit.speed;
            rows.push(Row { trip: trip.name, walker, plan, seconds, expanded, usec });
        }
    }
    Some(rows)
}

/// PathTrials.ground: the field's ground with the crag; None without the field's map.
pub fn ground(content: &Content) -> Option<Ground> {
    let mut ground = map::of("field", &content.tuning)?.ground(GroundTuning::of(&content.tuning));
    let crag = Paint { height: Some(CRAG_QUARTERS), climb: Some(CRAG_CLIMB), ..Paint::default() };
    ground.paint(CRAG, &crag);
    Some(ground)
}

/// The tuning a path's climb reads.
pub fn path_tuning(content: &Content) -> PathTuning {
    let tuning = &content.tuning;
    PathTuning {
        cliff_quarters: tuning.ground_cliff_quarters,
        stamina_climb: tuning.stamina_climb,
        stamina_swim: tuning.stamina_swim,
    }
}

/// PathTrials.report: a table of the trips, and the whole-field ways drawn over `ground`.
pub fn report(rows: &[Row], ground: &Ground) -> Vec<String> {
    let mut out = vec![format!(
        "{:<16} {:<18} {:>6} {:>5} {:>8} {:>8} {:>9} {:>9}",
        "trip", "walker", "cells", "legs", "reaches", "seconds", "searched", "search us"
    )];
    out.extend(rows.iter().map(line));
    for row in rows {
        let view = match row.trip {
            "across" => (FIELD_VIEW, 2),
            "stream" => (STREAM_VIEW, 1),
            _ => continue,
        };
        out.push(format!("\n{}, {}:", row.trip, row.walker));
        out.extend(drawing(&row.plan.cells, view.0, view.1, ground, &[CRAG]));
    }
    out
}

/// `view` of `ground` drawn a character a `scale` x `scale` block, with `cells` on it and
/// `cliffs` marked (PathTrials.drawing).
pub fn drawing(
    cells: &[Cell],
    view: Area,
    scale: i32,
    ground: &Ground,
    cliffs: &[Area],
) -> Vec<String> {
    let on_way: HashSet<Cell> = cells.iter().map(|&(x, y)| (x / scale, y / scale)).collect();
    let (x, y, width, height) = view;
    let mark = |block: Cell| {
        let cell = (block.0 * scale, block.1 * scale);
        if on_way.contains(&block) {
            '*'
        } else if cliffs.iter().any(|&cliff| holds(cliff, cell)) {
            '#'
        } else {
            ground_mark(ground, cell)
        }
    };
    let columns = x / scale..(x + width) / scale;
    (y / scale..(y + height) / scale)
        .map(|row| columns.clone().map(|column| mark((column, row))).collect())
        .collect()
}

/// A row's line: `"%-16s %-18s %6d %5d %8s %8.1f %9d %9d"`.
fn line(row: &Row) -> String {
    let legs = row.plan.waypoints.len().max(1) - 1;
    let reaches = if row.plan.reaches { "yes" } else { "edge" };
    format!(
        "{:<16} {:<18} {:>6} {:>5} {:>8} {:>8.1} {:>9} {:>9}",
        row.trip,
        row.walker,
        row.plan.cells.len(),
        legs,
        reaches,
        row.seconds,
        row.expanded,
        row.usec
    )
}

/// A trip as one walker makes it.
struct Way<'a> {
    walker: &'a PathWalker,
    trip: &'a Trip,
    route: &'a Route,
    sight: Option<&'a Sight>,
}

impl Way<'_> {
    fn from(&self) -> V2 {
        V2::new(self.trip.from.0, self.trip.from.1)
    }

    /// The cells the search searched, and the mean time (microseconds) of `repeats`.
    fn timed(&self, over: &mut Over, repeats: i64) -> (i64, i64) {
        let (from, times) = (self.from(), repeats.max(1));
        let began = Instant::now();
        let mut expanded = 0;
        for _ in 0..times {
            let eyes = Eyes { sight: self.sight, memory: None };
            let mut search = paths::search(over, self.walker, paths::cell_of(from), eyes);
            match self.trip.to {
                Some((x, y)) => search.toward_cell(paths::cell_of(V2::new(x, y))),
                None => search.onto_route(self.route, self.route.distance_of(from)),
            };
            expanded = search.expanded;
        }
        (expanded, began.elapsed().as_micros() as i64 / times)
    }

    /// The way planned (TerrainPaths.plan, or .rejoin onto the route).
    fn planned(&self, over: &mut Over) -> Plan {
        let eyes = Eyes { sight: self.sight, memory: None };
        match self.trip.to {
            Some((x, y)) => paths::plan(over, self.walker, self.from(), V2::new(x, y), eyes),
            None => paths::rejoin(over, self.walker, self.from(), self.route, -1.0, eyes),
        }
    }
}

/// What `unit` sees from `at` facing east over the ground (PathSight.of).
fn sight_of(unit: &Unit, at: V2, content: &Content) -> Sight {
    let reach = unit.detection_range;
    Sight {
        origin: at,
        facing: V2::new(1.0, 0.0),
        ahead: reach * unit.sight_ahead,
        side: reach * unit.sight_side,
        behind: reach * unit.sight_behind,
        hides: true,
        budget: content.tuning.sight_budget,
    }
}

/// `content` with the pack (PACK_WEIGHT, no slots) among its armour.
fn packed(content: &Content) -> Content {
    let mut out = content.clone();
    let item = Item { item_name: PACK.into(), weight: PACK_WEIGHT, ..Item::default() };
    out.armour.insert(PACK.into(), Armour { item, armour: 0, ward: 0 });
    out
}

/// PathTrials._unit: the walker's definition.
fn unit(content: &Content, walker: &str) -> Option<Unit> {
    let mut unit = content.units.get(GREM)?.clone();
    let traits = |pairs: &[(&str, i64)]| -> BTreeMap<String, i64> {
        pairs.iter().map(|&(id, level)| (id.to_string(), level)).collect()
    };
    match walker {
        "loaded militiaman" => {
            unit = content.units.get(MILITIA)?.clone();
            unit.items.push(format!("armour/{PACK}"));
        }
        "cart (sinks)" => unit.traits = traits(&[("sinks", 1), ("cant_climb", 1)]),
        "climber" => unit.traits = traits(&[("climber", CRAG_CLIMB as i64)]),
        _ => {}
    }
    Some(unit)
}

/// TerrainWalker.of `unit`.
fn walker_of(content: &Content, unit: &Unit) -> Walker {
    let [height, swimmer, climber, swims, climbs] = made::walker(unit, &Arms::of(content, unit));
    Walker {
        height,
        swimmer: swimmer as i64,
        climber: climber as i64,
        swims: swims != 0.0,
        climbs: climbs != 0.0,
    }
}

/// Rect2i.has_point.
fn holds((x, y, width, height): Area, (column, row): Cell) -> bool {
    column >= x && row >= y && column < x + width && row < y + height
}

/// PathTrials._ground_mark: the cell's ground as a bare walker 1 cell high crosses it.
fn ground_mark(ground: &Ground, cell: Cell) -> char {
    let centre = paths::centre(cell);
    let bare = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
    let pace = ground.crossing(&bare, centre, centre);
    if pace <= 0.0 {
        '~'
    } else if ground.blocks_sight(centre) {
        if pace >= 0.5 {
            'T'
        } else {
            '='
        }
    } else if pace < 1.0 {
        '='
    } else if ground.height_at(centre) > 0 {
        '^'
    } else {
        '.'
    }
}
