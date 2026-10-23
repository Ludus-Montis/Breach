//! Paths over the ground (spec 39 step 3c), from the gdUnit tests they replace:
//! test_terrain_paths, test_walker_shares, test_path_sight, test_route_rejoin, the sight
//! and plan scenarios of test_sight_obscurance, the standing and planning scenarios of
//! test_climb_stamina and test_formation_pathing's road. Those through a battle stay
//! gdUnit until their phases port.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::{GroundTuning, Paint};

/// A ground and its cache of shares, which a paint drops (as TerrainGround's does).
struct Field {
    ground: Ground,
    cache: Cache,
    tuning: PathTuning,
}

impl Field {
    fn new(cols: i32, rows: i32) -> Field {
        let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
        let t = &content.tuning;
        let tuning = PathTuning {
            cliff_quarters: t.ground_cliff_quarters,
            stamina_climb: t.stamina_climb,
            stamina_swim: t.stamina_swim,
        };
        Field {
            ground: Ground::new(cols, rows, GroundTuning::of(t)),
            cache: Cache::default(),
            tuning,
        }
    }

    fn paint(&mut self, area: (i32, i32, i32, i32), paint: Paint) -> &mut Field {
        self.ground.paint(area, &paint);
        self.cache.clear();
        self
    }

    fn over(&mut self) -> Over<'_> {
        Over { ground: &self.ground, cache: &mut self.cache, tuning: self.tuning }
    }

    fn cells(&mut self, walker: &PathWalker, from: V2, goal: V2) -> Vec<Cell> {
        search(&mut self.over(), walker, cell_of(from), Eyes::default()).toward_cell(cell_of(goal))
    }

    fn plan(&mut self, walker: &PathWalker, from: V2, goal: V2, eyes: Eyes) -> Plan {
        plan(&mut self.over(), walker, from, goal, eyes)
    }

    fn rejoin(
        &mut self,
        walker: &PathWalker,
        from: V2,
        route: &Route,
        progress: f64,
        eyes: Eyes,
    ) -> Plan {
        rejoin(&mut self.over(), walker, from, route, progress, eyes)
    }
}

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

/// TerrainWalker.new(height, swimmer, climber, swims, climbs).
fn walker(height: f64, swimmer: i64, climber: i64, swims: bool, climbs: bool) -> PathWalker {
    PathWalker::of(Walker { height, swimmer, climber, swims, climbs })
}

/// TerrainWalker.new(1.0): a cell tall, swimming and climbing at its implicit levels.
fn grem() -> PathWalker {
    walker(1.0, 0, 0, true, true)
}

fn grounded() -> PathWalker {
    walker(1.0, 0, 0, false, false)
}

fn cost_of(cost: f64) -> Paint {
    Paint { cost: Some(cost), ..Paint::default() }
}

fn depth(depth: f64) -> Paint {
    Paint { depth: Some(depth), ..Paint::default() }
}

fn height(height: i32) -> Paint {
    Paint { height: Some(height), ..Paint::default() }
}

fn cliff(height: i32, climb: i32) -> Paint {
    Paint { height: Some(height), climb: Some(climb), ..Paint::default() }
}

/// A wall too high for a climber at its implicit level.
fn wall() -> Paint {
    cliff(8, 3)
}

fn wood() -> Paint {
    Paint { cost: Some(0.5), obscurance: Some(1.0), ..Paint::default() }
}

fn touches(cells: &[Cell], (x, y, w, h): (i32, i32, i32, i32)) -> bool {
    cells.iter().any(|&(cx, cy)| cx >= x && cy >= y && cx < x + w && cy < y + h)
}

fn near(a: f64, b: f64, within: f64) {
    assert!((a - b).abs() <= within, "{a} is not within {within} of {b}");
}

/// A sight `ahead`, `side` and `behind` cells, over ground that hides with `budget`.
fn eyes(at: V2, way: V2, (ahead, side, behind): (f64, f64, f64), budget: Option<f64>) -> Sight {
    Sight {
        origin: at,
        facing: way.normalized(),
        ahead,
        side,
        behind,
        hides: budget.is_some(),
        budget: budget.unwrap_or(f64::INFINITY),
    }
}

fn shaped(at: V2, way: V2) -> Sight {
    eyes(at, way, (20.0, 10.0, 5.0), None)
}

fn seen(sight: &Sight) -> Eyes<'_> {
    Eyes { sight: Some(sight), memory: None }
}

// TerrainPaths

fn stream(ford: bool) -> Field {
    let mut field = Field::new(24, 12);
    field.paint((10, 0, 3, 12), depth(1.2));
    if ford {
        field.paint((10, 1, 3, 2), depth(0.4));
    }
    field
}

fn across(field: &mut Field, walker: &PathWalker) -> Vec<Cell> {
    field.cells(walker, v(2.5, 6.5), v(21.5, 6.5))
}

#[test]
fn a_unit_goes_round_a_wood_when_that_is_quicker() {
    let mut field = Field::new(20, 12);
    field.paint((8, 0, 4, 9), cost_of(0.5));
    let cells = field.cells(&grem(), v(2.5, 5.5), v(17.5, 5.5));

    assert!(!cells.is_empty());
    assert!(!touches(&cells, (8, 0, 4, 9)));
    assert_eq!(cells.last(), Some(&(17, 5)));
}

#[test]
fn a_ford_is_preferred_to_swimming_by_cost_alone() {
    let cells = across(&mut stream(true), &grem());

    assert!(touches(&cells, (10, 1, 3, 2)));
    assert!(!touches(&cells, (10, 3, 3, 9)));
    assert!(!touches(&cells, (10, 0, 3, 1)));
}

#[test]
fn with_no_ford_in_reach_a_unit_swims_and_a_non_swimmer_cannot_cross() {
    assert!(touches(&across(&mut stream(false), &grem()), (10, 0, 3, 12)));
    let sinks = walker(1.0, 0, 0, false, true);
    assert!(across(&mut stream(false), &sinks).is_empty());
    assert!(touches(&across(&mut stream(true), &sinks), (10, 1, 3, 2)));
}

#[test]
fn a_cliff_blocks_a_non_climber_and_slows_one_a_level_short() {
    let mut field = Field::new(20, 8);
    field.paint((10, 0, 10, 8), cliff(8, 2));
    let (start, goal) = (v(3.5, 3.5), v(15.5, 3.5));
    let short = walker(1.0, 0, 1, true, true);
    let able = walker(1.0, 0, 2, true, true);

    assert!(field.cells(&grem(), start, goal).is_empty());
    let slow = field.plan(&short, start, goal, Eyes::default()).waypoints;
    let quick = field.plan(&able, start, goal, Eyes::default()).waypoints;
    assert!(!slow.is_empty());
    assert!(cost(&field.ground, &short.walker, &slow) > cost(&field.ground, &able.walker, &quick));
}

#[test]
fn no_corner_is_cut_between_impassable_cells() {
    let mut field = Field::new(10, 10);
    field.paint((4, 4, 1, 1), cliff(8, 3)).paint((5, 5, 1, 1), cliff(8, 3));

    assert!(field.cells(&grem(), v(4.5, 5.5), v(5.5, 4.5)).len() > 2);
}

#[test]
fn open_ground_gives_a_straight_line_hugged_by_its_cells() {
    let mut field = Field::new(20, 10);
    let (start, goal) = (v(1.5, 1.5), v(13.5, 5.5));

    for cell in field.cells(&grem(), start, goal) {
        let at = centre(cell);
        let way = goal.sub(start);
        let t = (at.sub(start).dot(way) / way.dot(way)).clamp(0.0, 1.0);
        let nearest = v(start.x + way.x * t, start.y + way.y * t);
        assert!(at.distance_to(nearest) < 0.8);
    }
    assert_eq!(field.plan(&grem(), start, goal, Eyes::default()).waypoints, vec![start, goal]);
}

fn mirrorable(mirrored: bool) -> Field {
    let mut field = Field::new(24, 16);
    let areas = [(6, 2, 3, 8), (12, 0, 2, 11), (16, 10, 4, 4)];
    let paints = [cost_of(0.5), wall(), depth(1.2)];
    for (area, paint) in areas.into_iter().zip(paints) {
        let x = if mirrored { 24 - (area.0 + area.2) } else { area.0 };
        field.paint((x, area.1, area.2, area.3), paint);
    }
    field
}

#[test]
fn the_ground_mirrored_gives_the_path_mirrored() {
    let (start, goal) = (v(2.5, 6.5), v(21.5, 12.5));
    let flip = |at: V2| v(24.0 - at.x, at.y);
    let cells = mirrorable(false).cells(&grem(), start, goal);
    let mirrored = mirrorable(true).cells(&grem(), flip(start), flip(goal));

    assert!(!cells.is_empty());
    let flipped: Vec<Cell> = cells.iter().map(|&(x, y)| (23 - x, y)).collect();
    assert_eq!(mirrored, flipped);
    let way = mirrorable(false).plan(&grem(), start, goal, Eyes::default()).waypoints;
    let back = mirrorable(true).plan(&grem(), flip(start), flip(goal), Eyes::default()).waypoints;
    assert_eq!(back.len(), way.len());
    for (one, other) in way.iter().zip(&back) {
        assert!(flip(*one).distance_to(*other) < 0.0001);
    }
}

#[test]
fn the_same_ground_gives_the_same_path_whatever_was_painted_first() {
    let mut first = Field::new(24, 16);
    first.paint((6, 2, 3, 8), cost_of(0.5)).paint((12, 0, 2, 11), wall());
    let mut second = Field::new(24, 16);
    second.paint((12, 0, 2, 11), wall()).paint((6, 2, 3, 8), cost_of(0.5));
    let (start, goal) = (v(2.5, 6.5), v(21.5, 12.5));

    let path = first.cells(&grem(), start, goal);
    assert_eq!(second.cells(&grem(), start, goal), path);
    assert_eq!(first.cells(&grem(), start, goal), path);
}

#[test]
fn the_path_search_still_finds_a_road_faster_than_open_ground() {
    let mut field = Field::new(40, 16);
    field.paint((0, 2, 40, 1), cost_of(3.0));
    let cells = field.cells(&grem(), v(1.5, 8.5), v(38.5, 8.5));

    assert!(cells.iter().any(|&(_, y)| y == 2));
    assert_eq!(field.ground.fastest(), 3.0);
}

// The shares

fn sheltered() -> Field {
    let mut field = Field::new(8, 6);
    let slope = Paint { cost: Some(0.5), height: Some(2), ..Paint::default() };
    field.paint((3, 0, 1, 6), slope).paint((5, 0, 1, 6), depth(1.2)).paint((6, 0, 2, 6), height(8));
    field
}

#[test]
fn a_share_looked_up_is_the_crossing_and_none_leaves_the_grid() {
    let mut field = sheltered();
    let one = grem().walker;
    let shares = field.cache.of(&field.ground, &one);

    for index in 0..8 * 6 {
        let (x, y) = (index as i32 % 8, index as i32 / 8);
        for (step, (dx, dy)) in shares::STEPS.iter().enumerate() {
            let (nx, ny) = (x + dx, y + dy);
            let inside = nx >= 0 && ny >= 0 && nx < 8 && ny < 6;
            let expected = if inside {
                field.ground.crossing(&one, centre((x, y)), centre((nx, ny)))
            } else {
                0.0
            };
            near(shares.share(&field.ground, index, step), expected, 0.000001);
            near(shares.share(&field.ground, index, step), expected, 0.000001); // looked up
        }
    }
}

#[test]
fn walkers_of_one_kind_share_their_shares_until_the_cache_is_dropped() {
    let mut field = sheltered();
    field.cache.of(&field.ground, &grem().walker);
    field.cache.of(&field.ground, &grem().walker);
    assert_eq!(field.cache.len(), 1);
    field.cache.of(&field.ground, &walker(1.0, 0, 2, true, true).walker);
    field.cache.of(&field.ground, &walker(1.5, 0, 0, true, true).walker);
    assert_eq!(field.cache.len(), 3);
    let tall = f64::from_bits(1.0f64.to_bits() + 1); // one ulp above 1: the same written to a few places
    field.cache.of(&field.ground, &walker(tall, 0, 0, true, true).walker);
    assert_eq!(field.cache.len(), 4);
    assert!(field.cache.of(&field.ground, &grem().walker).share(&field.ground, 8 + 2, 0) < 1.0);

    let flat = Paint { cost: Some(1.0), height: Some(0), ..Paint::default() };
    field.paint((3, 0, 1, 6), flat);
    assert!(field.cache.is_empty());
    assert_eq!(field.cache.of(&field.ground, &grem().walker).share(&field.ground, 8 + 2, 0), 1.0);
}

// Sight

#[test]
fn sight_has_a_shape_far_ahead_near_behind() {
    let sight = shaped(V2::ZERO, v(1.0, 0.0));

    assert!(sight.sees(None, v(19.5, 0.0)));
    assert!(!sight.sees(None, v(20.5, 0.0)));
    assert!(sight.sees(None, v(0.0, 9.5)));
    assert!(!sight.sees(None, v(0.0, -10.5)));
    assert!(sight.sees(None, v(-4.5, 0.0)));
    assert!(!sight.sees(None, v(-5.5, 0.0)));
    near(sight.reach(v(1.0, 1.0).normalized()), 15.0, 0.0001);
}

#[test]
fn a_unit_sees_a_few_cells_into_a_wood_then_nothing() {
    let mut field = Field::new(40, 20);
    field.paint((10, 0, 10, 20), wood());
    let ground = Some(&field.ground);
    let wide = (30.0, 30.0, 30.0);
    let sight = eyes(v(5.5, 5.5), v(1.0, 0.0), wide, Some(3.0));

    assert!(sight.sees(ground, v(9.5, 5.5)));
    assert!(sight.sees(ground, v(12.5, 5.5))); // 2.5 cells in
    assert!(!sight.sees(ground, v(13.5, 5.5))); // 3.5 in
    assert!(!sight.sees(ground, v(25.5, 5.5))); // beyond it
    let inside = eyes(v(15.5, 5.5), v(1.0, 0.0), wide, Some(3.0));
    assert!(inside.sees(ground, v(17.5, 5.5)));
    assert!(!inside.sees(ground, v(19.5, 5.5)));
}

#[test]
fn fog_shortens_sight_rather_than_blocking_it() {
    let clear = Field::new(60, 10);
    let mut weather = Field::new(60, 10);
    weather.ground.set_fog(0.1);
    let mut bank = Field::new(60, 10);
    bank.paint((0, 0, 60, 10), Paint { obscurance: Some(0.1), ..Paint::default() });
    let sight = eyes(v(1.5, 5.5), v(1.0, 0.0), (30.0, 30.0, 30.0), Some(3.0));

    assert!(sight.sees(Some(&clear.ground), v(29.5, 5.5)));
    for fogged in [&weather, &bank] {
        assert!(sight.sees(Some(&fogged.ground), v(20.5, 5.5)));
        assert!(sight.sees(Some(&fogged.ground), v(29.5, 5.5)));
        assert!(!sight.sees(Some(&fogged.ground), v(32.5, 5.5)));
    }
}

const START: V2 = V2 { x: 5.5, y: 4.5 };
const GOAL: V2 = V2 { x: 35.5, y: 4.5 };

/// A wall across the north of the field, x 20-21, open to the south from y 22.
fn walled(mirrored: bool) -> Field {
    let mut field = Field::new(40, 30);
    field.paint((if mirrored { 18 } else { 20 }, 0, 2, 22), wall());
    field
}

#[test]
fn a_goal_in_sight_is_reached_and_one_beyond_is_planned_towards() {
    let mut open = Field::new(40, 30);
    let sight = shaped(START, v(1.0, 0.0));
    let near_goal = open.plan(&grem(), START, v(15.5, 4.5), seen(&sight));
    let far = open.plan(&grem(), START, GOAL, seen(&sight));

    assert!(near_goal.reaches);
    assert_eq!(near_goal.waypoints, vec![START, v(15.5, 4.5)]);
    assert!(!far.reaches);
    let edge = far.cells.last().unwrap();
    assert_eq!(edge.1, 4); // straight on, to the edge of sight ahead
    assert!(edge.0 > 22);
}

#[test]
fn what_it_cannot_see_counts_as_open() {
    let mut blind = Field::new(60, 12);
    blind.paint((40, 0, 2, 11), wall()); // a wall far ahead, its gap at y 11
    let (start, goal) = (v(2.5, 5.5), v(57.5, 5.5));
    let sight = shaped(start, v(1.0, 0.0));
    let planned = blind.plan(&grem(), start, goal, seen(&sight));

    assert!(!planned.reaches);
    assert!(planned.cells.iter().all(|&(_, y)| y == 5));
    assert!(touches(&blind.cells(&grem(), start, goal), (40, 11, 2, 1)));
}

#[test]
fn a_wall_is_felt_along_until_the_gap_comes_into_view() {
    let mut ground = walled(false);
    let sight = shaped(START, v(1.0, 0.0));
    let first = ground.plan(&grem(), START, GOAL, seen(&sight));
    assert!(!first.reaches);
    let edge = *first.cells.last().unwrap();
    assert!(edge.0 < 20);
    assert!(edge.1 > 4); // along the wall, south, the only way open

    let mut walked = Vec::new();
    let (mut at, mut way) = (START, v(1.0, 0.0));
    let mut memory = Memory::default();
    let mut reaches = false;
    for _attempt in 0..12 {
        let sight = shaped(at, way);
        let eyes = Eyes { sight: Some(&sight), memory: Some(&mut memory) };
        let planned = ground.plan(&grem(), at, GOAL, eyes);
        walked.extend_from_slice(&planned.cells);
        reaches = planned.reaches;
        if reaches {
            break;
        }
        let points = &planned.waypoints;
        let (last, before) = (points[points.len() - 1], points[points.len() - 2]);
        way = last.sub(before).normalized();
        at = last;
    }
    assert!(reaches);
    assert!(touches(&walked, (20, 22, 2, 8)));
}

#[test]
fn mirrored_ground_and_sight_give_the_plan_mirrored() {
    let flip = |at: V2| v(40.0 - at.x, at.y);
    let sight = shaped(START, v(1.0, 0.0));
    let plan = walled(false).plan(&grem(), START, GOAL, seen(&sight));
    let other = shaped(flip(START), v(-1.0, 0.0));
    let back = walled(true).plan(&grem(), flip(START), flip(GOAL), seen(&other));

    let flipped: Vec<Cell> = plan.cells.iter().map(|&(x, y)| (39 - x, y)).collect();
    assert_eq!(back.cells, flipped);
}

fn lane() -> Route {
    Route::new(vec![v(0.0, 20.5), v(40.0, 20.5)], 0.0)
}

#[test]
fn rejoining_a_route_plans_to_the_nearest_place_on_it_in_sight() {
    let mut open = Field::new(40, 30);
    let from = v(10.5, 10.5);
    let sight = shaped(from, v(0.0, 1.0));
    let back = open.rejoin(&grem(), from, &lane(), -1.0, seen(&sight));

    assert!(back.reaches);
    assert_eq!(back.cells.last(), Some(&(10, 20)));
    near(back.along, 10.5, 0.0001);
    let mut walled = Field::new(40, 30);
    walled.paint((0, 15, 15, 2), wall());
    let round = walled.rejoin(&grem(), from, &lane(), -1.0, seen(&sight));
    assert!(round.reaches);
    assert!(touches(&round.cells, (15, 15, 25, 2)));
}

#[test]
fn on_the_route_it_is_already_there_and_out_of_sight_it_heads_for_it() {
    let mut open = Field::new(40, 30);
    let on = v(10.5, 20.5);
    let sight = shaped(on, v(1.0, 0.0));
    let there = open.rejoin(&grem(), on, &lane(), -1.0, seen(&sight));
    let from = v(10.5, 3.5);
    let short = eyes(from, v(1.0, 0.0), (6.0, 3.0, 2.0), None);
    let toward = open.rejoin(&grem(), from, &lane(), -1.0, seen(&short));

    assert!(there.reaches);
    assert_eq!(there.cells, vec![(10, 20)]);
    assert!(!toward.reaches);
    assert_eq!(toward.along, -1.0);
    assert!(toward.cells.last().unwrap().1 > 4);
}

// Rejoining a route

const FROM: V2 = V2 { x: 20.5, y: 14.5 };
const LEFT_AT: f64 = 20.5; // how far along the route the group was when it left it

fn east() -> Route {
    Route::new(vec![v(0.0, 20.5), v(60.0, 20.5)], 0.0)
}

/// A river, too deep for a grem that can't swim, cutting the group off from the route
/// below it and ahead (it crosses the route there): open only to the west, behind it.
fn cut_off(rows_first: bool) -> Field {
    let mut field = Field::new(60, 30);
    let mut areas = vec![(24, 0, 2, 30), (16, 17, 8, 2), (16, 19, 2, 11)];
    if rows_first {
        areas.reverse();
    }
    for area in areas {
        field.paint(area, depth(2.0));
    }
    field
}

fn back_on(field: &mut Field, walker: &PathWalker) -> Plan {
    field.rejoin(walker, FROM, &east(), LEFT_AT, Eyes::default())
}

#[test]
fn on_open_ground_it_rejoins_at_or_just_ahead_of_where_it_left() {
    let back = back_on(&mut Field::new(60, 30), &grem());

    assert!(back.reaches);
    assert!((20.0..=22.0).contains(&back.along));
}

#[test]
fn round_a_wall_it_rejoins_beyond_the_far_end_ahead() {
    let mut field = Field::new(60, 30);
    field.paint((13, 17, 15, 2), wall()); // centred on the group: either end as near
    let back = back_on(&mut field, &grem());

    assert!(back.reaches);
    assert!(back.along > 27.0);
}

#[test]
fn cut_off_by_a_river_it_doubles_back() {
    let back = back_on(&mut cut_off(false), &grounded());

    assert!(back.reaches);
    assert!(back.along < 16.0);
}

#[test]
fn the_same_ground_gives_the_same_rejoin_whatever_was_painted_first() {
    let one = back_on(&mut cut_off(true), &grounded());
    assert_eq!(one.cells, back_on(&mut cut_off(false), &grounded()).cells);
}

#[test]
fn the_ground_and_route_mirrored_give_the_rejoin_mirrored() {
    let mut field = Field::new(60, 30);
    field.paint((14, 17, 12, 2), wall());
    let mut mirrored = Field::new(60, 30);
    mirrored.paint((60 - 26, 17, 12, 2), wall());
    let west = Route::new(vec![v(60.0, 20.5), v(0.0, 20.5)], 0.0);
    let there = back_on(&mut field, &grem());
    let back = mirrored.rejoin(&grem(), v(60.0 - FROM.x, FROM.y), &west, LEFT_AT, Eyes::default());

    near(back.along, there.along, 0.0001);
    let flipped: Vec<Cell> = there.cells.iter().map(|&(x, y)| (59 - x, y)).collect();
    assert_eq!(back.cells, flipped);
}

#[test]
fn with_no_way_to_the_route_there_is_no_rejoin() {
    let mut field = Field::new(60, 30);
    field.paint((0, 17, 60, 2), wall());
    let back = back_on(&mut field, &grem());

    assert!(!back.reaches);
    assert!(back.cells.is_empty());
}

#[test]
fn the_search_stops_at_the_first_point_ahead_so_it_stays_small() {
    let route = east();
    let mut open = Field::new(60, 30);
    let mut over = open.over();
    let mut first = search(&mut over, &grem(), (20, 14), Eyes::default());
    first.onto_route(&route, LEFT_AT);
    let open_count = first.expanded;
    let mut walled = Field::new(60, 30);
    walled.paint((13, 17, 15, 2), wall());
    let mut over = walled.over();
    let mut round = search(&mut over, &grem(), (20, 14), Eyes::default());
    round.onto_route(&route, LEFT_AT);

    assert!(open_count <= 30);
    assert!(round.expanded < 200);
}

// Obscuring ground

#[test]
fn a_plan_cannot_see_through_a_wood_and_remembers_what_it_walked_through() {
    let mut field = Field::new(40, 20);
    field.paint((10, 0, 10, 20), wood()).paint((25, 0, 2, 19), wall()); // a wall past the wood
    let wide = (30.0, 30.0, 30.0);
    let mut memory = Memory::default();
    let (start, goal) = (v(5.5, 5.5), v(35.5, 5.5));

    let sight = eyes(start, v(1.0, 0.0), wide, Some(3.0));
    let outside =
        field.plan(&grem(), start, goal, Eyes { sight: Some(&sight), memory: Some(&mut memory) });
    assert!(!outside.reaches); // the wall is hidden: straight on into the wood
    assert!(outside.cells.iter().all(|&(_, y)| y == 5));
    let past = v(21.5, 5.5); // through the wood: the wall comes into view
    let sight = eyes(past, v(1.0, 0.0), wide, Some(3.0));
    field.plan(&grem(), past, goal, Eyes { sight: Some(&sight), memory: Some(&mut memory) });
    assert!(memory.knows(5 * 40 + 25, 40 * 20)); // the wall, seen
    assert!(memory.knows(5 * 40 + 11, 40 * 20)); // the wood walked in

    let sight = eyes(start, v(1.0, 0.0), wide, Some(3.0));
    let again =
        field.plan(&grem(), start, goal, Eyes { sight: Some(&sight), memory: Some(&mut memory) });
    assert!(!again.reaches);
    assert!(again.cells.last().unwrap().1 > 5); // heading for the way round the wall
}

#[test]
fn a_plan_through_obscuring_ground_mirrored_is_the_plan_mirrored() {
    let mut field = Field::new(40, 24);
    let mut mirrored = Field::new(40, 24);
    for (x, y, w, h) in [(12, 3, 6, 12), (24, 10, 3, 14)] {
        field.paint((x, y, w, h), wood());
        mirrored.paint((40 - (x + w), y, w, h), wood());
    }
    let (start, goal) = (v(4.5, 8.5), v(35.5, 14.5));
    let flip = |at: V2| v(40.0 - at.x, at.y);
    let wide = (30.0, 30.0, 30.0);
    let sight = eyes(start, v(1.0, 0.0), wide, Some(3.0));
    let plan = field.plan(&grem(), start, goal, seen(&sight));
    let other = eyes(flip(start), v(-1.0, 0.0), wide, Some(3.0));
    let back = mirrored.plan(&grem(), flip(start), flip(goal), seen(&other));

    let flipped: Vec<Cell> = plan.cells.iter().map(|&(x, y)| (39 - x, y)).collect();
    assert_eq!(back.cells, flipped);
}

// Climbing on stamina

/// A staircase of cliffs, a cell a step, up from x 10 to a plateau at x 14-19 on rows 0-9;
/// down its east side a cliff to a 2-cell ledge at x 20-21, and another to the ground; a
/// wall too high to climb across rows 10-11.
fn stairs(mirrored: bool) -> Field {
    let mut field = Field::new(30, 24);
    let columns = [(10, 8), (11, 16), (12, 24), (13, 32), (14, 40), (15, 40), (16, 40), (17, 40)];
    let more = [(18, 40), (19, 40), (20, 16), (21, 16)];
    for (x, level) in columns.into_iter().chain(more) {
        field.paint((if mirrored { 29 - x } else { x }, 0, 1, 10), height(level));
    }
    field.paint((0, 10, 30, 2), cliff(80, 9));
    field
}

fn climber(stamina: f64) -> PathWalker {
    PathWalker { stamina, ..walker(1.0, 0, 1, true, true) }
}

fn stands(field: &Field, at: V2) -> bool {
    climb::standable(&field.ground, &grem().walker, at, field.tuning.cliff_quarters)
}

#[test]
fn a_unit_stands_where_the_slope_across_the_cell_is_under_the_threshold() {
    let field = stairs(false);

    assert!(stands(&field, v(9.5, 5.5))); // the foot
    assert!(!stands(&field, v(11.5, 5.5))); // a face
    assert!(stands(&field, v(14.5, 5.5))); // the top
    assert!(stands(&field, v(20.5, 5.5))); // a ledge
    assert!(!stands(&field, v(13.5, 5.5)));
    let mut water = Field::new(8, 8);
    water.paint((2, 2, 3, 3), depth(1.5));
    assert!(!stands(&water, v(3.5, 3.5)));
    assert!(stands(&water, v(0.5, 0.5)));
}

#[test]
fn standing_is_the_same_on_the_ground_mirrored() {
    let (field, mirrored) = (stairs(false), stairs(true));
    for x in 0..30 {
        let here = stands(&field, v(x as f32 + 0.5, 5.5));
        assert_eq!(stands(&mirrored, v(29.5 - x as f32, 5.5)), here);
    }
}

#[test]
fn a_plan_climbs_only_what_its_stamina_lasts_between_ledges() {
    let mut field = stairs(false);
    let (start, goal) = (v(8.5, 5.5), v(16.5, 5.5));
    // each step up: 1 cell at the climb pace (0.25), climber 1 against difficulty 1 - 4 s
    // at 4 stamina a second; the stretch from the foot to the top is 5 steps, 80 stamina
    let fresh = field.cells(&climber(100.0), start, goal);
    let tired = field.cells(&climber(60.0), start, goal);

    assert!(fresh.contains(&(12, 5)));
    assert!(tired.is_empty()); // no way up it can last, and no way round
    let east = field.cells(&climber(20.0), v(25.5, 5.5), goal);
    assert!(east.contains(&(20, 5)) || east.contains(&(21, 5))); // by the ledge
}
