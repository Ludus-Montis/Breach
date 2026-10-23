//! The ground's scenarios (spec 39 step 3a), from the gdUnit tests they replace
//! (test_formation_terrain, test_terrain_walker's crossings, test_sight_obscurance,
//! test_ground_surface, test_ground_relief).

use super::surface::Surface;
use super::*;
use crate::content::tests::repo_content;
use crate::content::{Channel, Content, MapLayout, Terrain, TerrainLibrary, Tile};

fn tuning() -> GroundTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    GroundTuning::of(&content.tuning)
}

fn ground(cols: i32, rows: i32) -> Ground {
    Ground::new(cols, rows, tuning())
}

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

/// A unit of `height` with no climbing or swimming of its own (TerrainWalker.grounded).
fn grounded(height: f64) -> Walker {
    Walker { height, swimmer: 0, climber: 0, swims: false, climbs: false }
}

/// TerrainWalker.new(height, swimmer, climber): it may swim and climb.
fn walker(height: f64, swimmer: i64, climber: i64) -> Walker {
    Walker { height, swimmer, climber, swims: true, climbs: true }
}

fn paint(ground: &mut Ground, area: (i32, i32, i32, i32), paint: Paint) {
    ground.paint(area, &paint);
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 0.0001, "{a} is not {b}");
}

#[test]
fn open_ground_is_full_pace_and_a_wood_is_half() {
    let mut g = ground(64, 32);
    paint(&mut g, (10, 0, 5, 5), Paint { cost: Some(0.5), ..Paint::default() });
    assert_eq!(g.crossing(&grounded(1.0), v(2.5, 2.5), v(3.5, 2.5)), 1.0);
    assert_eq!(g.crossing(&grounded(1.0), v(9.5, 2.5), v(10.5, 2.5)), 0.5);
    assert_eq!(g.crossing(&grounded(1.0), v(-5.0, -5.0), v(-4.0, -5.0)), 1.0);
}

#[test]
fn uphill_is_slower_and_downhill_no_faster() {
    let mut g = ground(64, 32);
    paint(&mut g, (10, 0, 1, 5), Paint { height: Some(2), ..Paint::default() });
    near(g.crossing(&grounded(1.0), v(9.5, 1.0), v(10.5, 1.0)), 0.8);
    assert_eq!(g.crossing(&grounded(1.0), v(10.5, 1.0), v(11.5, 1.0)), 1.0);
    assert!(g.high_ground(v(10.5, 1.0), v(9.5, 1.0)));
    assert!(!g.high_ground(v(9.5, 1.0), v(10.5, 1.0)));
}

#[test]
fn a_rise_of_more_than_a_cell_is_a_cliff() {
    let mut g = ground(64, 32);
    paint(&mut g, (10, 0, 1, 5), Paint { height: Some(5), ..Paint::default() });
    assert_eq!(g.crossing(&grounded(1.0), v(9.5, 1.0), v(10.5, 1.0)), 0.0);
    assert_eq!(g.mode(&grounded(1.0), v(9.5, 1.0), v(10.5, 1.0)), Mode::Climbing);
}

#[test]
fn water_slows_by_depth_against_height() {
    let mut g = ground(64, 32);
    for (x, depth) in [(10, 0.1), (11, 0.3), (12, 0.7), (13, 1.2)] {
        paint(&mut g, (x, 0, 1, 1), Paint { depth: Some(depth), ..Paint::default() });
    }
    let at = |g: &Ground, x: i32, height: f64| {
        g.crossing(&grounded(height), v(x as f32 - 0.5, 0.5), v(x as f32 + 0.5, 0.5))
    };
    let t = tuning();
    assert_eq!(at(&g, 10, 1.0), 1.0);
    near(at(&g, 11, 1.0), t.wading);
    near(at(&g, 12, 1.0), t.slow_wading);
    assert_eq!(at(&g, 13, 1.0), 0.0);
    near(at(&g, 13, 2.0), t.slow_wading); // a brute, twice as tall, wades it
    assert_eq!(g.mode(&grounded(1.0), v(12.5, 0.5), v(13.5, 0.5)), Mode::Swimming);
    assert_eq!(g.mode(&grounded(2.0), v(12.5, 0.5), v(13.5, 0.5)), Mode::Walking);
}

const BELOW: V2 = V2 { x: 7.5, y: 2.5 };
const INTO: V2 = V2 { x: 8.5, y: 2.5 };

fn water(flows: i32) -> Ground {
    let mut g = ground(16, 8);
    paint(&mut g, (8, 0, 1, 8), Paint { depth: Some(1.2), flows: Some(flows), ..Paint::default() });
    g
}

fn cliff(climb: Option<i32>) -> Ground {
    let mut g = ground(16, 8);
    paint(&mut g, (8, 0, 8, 8), Paint { height: Some(8), climb, ..Paint::default() });
    g
}

#[test]
fn flowing_water_meets_the_swimmer_by_the_pair_rule() {
    let swim = tuning().swim_pace;
    near(water(0).crossing(&walker(1.0, 0, 0), BELOW, INTO), swim);
    assert_eq!(water(0).crossing(&grounded(1.0), BELOW, INTO), 0.0);
    let g = water(2);
    assert_eq!(g.crossing(&walker(1.0, 0, 0), BELOW, INTO), 0.0);
    near(g.crossing(&walker(1.0, 1, 0), BELOW, INTO), swim * 0.5);
    near(g.crossing(&walker(1.0, 2, 0), BELOW, INTO), swim);
    near(g.crossing(&walker(1.0, 5, 0), BELOW, INTO), swim);
}

#[test]
fn a_cliff_meets_the_climber_by_the_pair_rule() {
    let climb = tuning().climb_pace;
    let g = cliff(Some(2));
    assert_eq!(g.crossing(&walker(1.0, 0, 0), BELOW, INTO), 0.0);
    near(g.crossing(&walker(1.0, 0, 1), BELOW, INTO), climb * 0.5);
    near(g.crossing(&walker(1.0, 0, 2), BELOW, INTO), climb);
    assert_eq!(g.crossing(&walker(1.0, 0, 0), INTO, BELOW), 1.0);
    // A plain cliff demands ground_climb_demand (1): everyone climbs it at half pace.
    let plain = cliff(None);
    near(plain.crossing(&walker(1.0, 0, 0), BELOW, INTO), climb * 0.5);
    assert_eq!(plain.crossing(&grounded(1.0), BELOW, INTO), 0.0);
}

#[test]
fn a_walker_that_may_not_use_a_mode_does_not() {
    let cart = Walker { swims: false, climbs: false, ..walker(1.0, 0, 0) };
    assert_eq!(water(0).crossing(&cart, BELOW, INTO), 0.0);
    assert_eq!(cliff(Some(0)).crossing(&cart, BELOW, INTO), 0.0);
    let sinks = Walker { swims: false, ..walker(1.0, 0, 0) };
    assert!(cliff(Some(0)).crossing(&sinks, BELOW, INTO) > 0.0);
    let cant_climb = Walker { climbs: false, ..walker(1.0, 0, 0) };
    assert!(water(0).crossing(&cant_climb, BELOW, INTO) > 0.0);
}

#[test]
fn the_fastest_ground_is_the_dearest_cost_painted() {
    let mut g = ground(8, 8);
    assert_eq!(g.fastest(), 1.0);
    paint(&mut g, (0, 0, 2, 2), Paint { cost: Some(1.5), ..Paint::default() });
    paint(&mut g, (4, 4, 2, 2), Paint { cost: Some(0.5), ..Paint::default() });
    assert_eq!(g.fastest(), 1.5);
    paint(&mut g, (40, 40, 2, 2), Paint { cost: Some(3.0), ..Paint::default() }); // off it
    assert_eq!(g.fastest(), 1.5);
}

#[test]
fn a_painted_cell_blocks_sight_until_painted_clear() {
    let mut g = ground(8, 8);
    paint(&mut g, (2, 2, 2, 2), Paint { blocks_sight: Some(true), ..Paint::default() });
    assert!(g.blocks_sight(v(2.5, 3.5)));
    assert!(!g.blocks_sight(v(1.5, 3.5)));
    assert!(!g.blocks_sight(v(-1.5, 3.5)));
    paint(&mut g, (2, 2, 1, 1), Paint { blocks_sight: Some(false), ..Paint::default() });
    assert!(!g.blocks_sight(v(2.5, 2.5)));
    assert_eq!(g.height_at(v(2.5, 2.5)), 0);
}

fn obscured(value: f64) -> Paint {
    Paint { obscurance: Some(value), ..Paint::default() }
}

#[test]
fn a_sight_line_sums_obscurance_by_the_distance_through_each_cell() {
    let mut g = ground(40, 20);
    paint(&mut g, (10, 0, 10, 20), obscured(1.0));
    let along = |a: V2, b: V2| g.sight_along(a, b, f64::INFINITY);
    near(along(v(2.5, 5.5), v(8.5, 5.5)), 0.0);
    near(along(v(2.5, 5.5), v(12.5, 5.5)), 2.5);
    near(along(v(2.5, 5.5), v(30.5, 5.5)), 10.0);
    near(along(v(5.0, 0.0), v(25.0, 20.0)), 10.0 * 2f64.sqrt()); // 10 cells across at 45
}

#[test]
fn sight_sees_a_few_cells_into_a_wood_then_nothing() {
    let mut g = ground(40, 20);
    paint(&mut g, (10, 0, 10, 20), Paint { cost: Some(0.5), ..obscured(1.0) });
    let eye = v(5.5, 5.5);
    assert!(g.sight_clear(eye, v(9.5, 5.5), 3.0));
    assert!(g.sight_clear(eye, v(12.5, 5.5), 3.0)); // 2.5 cells in
    assert!(!g.sight_clear(eye, v(13.5, 5.5), 3.0)); // 3.5 in
    assert!(!g.sight_clear(eye, v(25.5, 5.5), 3.0)); // beyond it
}

#[test]
fn fog_shortens_sight_rather_than_blocking_it() {
    let clear = ground(60, 10);
    let mut weather = ground(60, 10);
    weather.set_fog(0.1);
    let mut bank = ground(60, 10);
    paint(&mut bank, (0, 0, 60, 10), obscured(0.1));
    let at = v(1.5, 5.5);
    assert!(clear.sight_clear(at, v(29.5, 5.5), 3.0));
    for fogged in [&weather, &bank] {
        assert!(fogged.sight_clear(at, v(20.5, 5.5), 3.0));
        assert!(fogged.sight_clear(at, v(29.5, 5.5), 3.0));
        assert!(!fogged.sight_clear(at, v(32.5, 5.5), 3.0));
    }
    near(weather.obscurance_at(v(-4.0, 5.0)), 0.1);
}

#[test]
fn a_sight_line_is_the_same_either_way_and_mirrored() {
    let mut g = ground(30, 30);
    let mut mirrored = ground(30, 30);
    for (x, y, w, h) in [(8, 3, 5, 9), (17, 14, 6, 4), (4, 20, 3, 3)] {
        paint(&mut g, (x, y, w, h), obscured(0.7));
        paint(&mut mirrored, (30 - (x + w), y, w, h), obscured(0.7));
    }
    let flip = |at: V2| v(30.0 - at.x, at.y);
    let lines = [
        (v(1.5, 1.5), v(28.5, 25.5)),
        (v(10.0, 0.0), v(10.0, 29.0)), // along a grid line
        (v(2.0, 22.0), v(26.0, 2.0)),  // through corners
        (v(15.5, 15.5), v(5.25, 7.75)),
    ];
    for (a, b) in lines {
        let there = g.sight_along(a, b, f64::INFINITY);
        near(g.sight_along(b, a, f64::INFINITY), there);
        near(mirrored.sight_along(flip(a), flip(b), f64::INFINITY), there);
    }
}

#[test]
fn obscurance_does_not_depend_on_what_was_painted_first() {
    let wood = Paint { cost: Some(0.5), ..obscured(1.0) };
    let hill = Paint { height: Some(4), ..Paint::default() };
    let mut first = ground(20, 20);
    paint(&mut first, (2, 2, 8, 8), wood.clone());
    paint(&mut first, (5, 5, 8, 8), hill.clone());
    let mut second = ground(20, 20);
    paint(&mut second, (5, 5, 8, 8), hill);
    paint(&mut second, (2, 2, 8, 8), wood);
    let (a, b) = (v(0.5, 0.5), v(19.5, 17.5));
    assert_eq!(second.sight_along(a, b, f64::INFINITY), first.sight_along(a, b, f64::INFINITY));
}

#[test]
fn a_line_running_along_a_cell_edge_takes_the_mean_of_either_side() {
    let mut g = ground(10, 10);
    paint(&mut g, (0, 0, 10, 5), obscured(1.0));
    near(g.obscurance_at(v(3.5, 5.0)), 0.5);
    near(g.obscurance_at(v(5.0, 5.0)), 0.5);
    near(g.sight_along(v(0.0, 5.0), v(10.0, 5.0), f64::INFINITY), 5.0);
}

// The surface (GroundSurface), its relief (GroundRelief) and channels (GroundChannels).

/// A row of tiles with these elevations (the default terrain sits at 0).
fn row(elevations: &[i64], ceiling: i64) -> Surface {
    let library = TerrainLibrary {
        terrains: vec![Terrain { id: "FIELDS".into(), ..Terrain::default() }],
        ..TerrainLibrary::default()
    };
    let layout = MapLayout {
        cols: elevations.len() as i64,
        rows: 1,
        default_terrain_id: "FIELDS".into(),
        ceiling,
        tiles: elevations
            .iter()
            .enumerate()
            .map(|(col, &elevation)| Tile { cell: [col as i64, 0], elevation, ..Tile::default() })
            .collect(),
        ..MapLayout::default()
    };
    Surface::new(&layout, &library)
}

#[test]
fn flat_ground_has_full_surface_cells_just_below_its_height() {
    let g = row(&[4, 4], 64);
    assert_eq!(g.surface_level([20, 5]), 3);
    assert_eq!(g.surface_shape([20, 5]), [4, 4, 4, 4]);
    assert_eq!(g.surface_height([20, 5]), 4.0);
}

#[test]
fn a_tile_is_64_cells_across() {
    assert_eq!(row(&[0, 64], 128).surface_height([96, 0]), 64.0); // tile 1's centre
}

#[test]
fn a_slope_runs_through_partial_cells() {
    let g = row(&[0, 16], 64); // a quarter of a cell per cell, between the tile centres
    assert_eq!(g.surface_level([32, 0]), 0);
    assert_eq!(g.surface_shape([32, 0]), [0, 1, 1, 0]);
    assert_eq!(g.surface_height([32, 0]), 0.125);
}

#[test]
fn digging_a_surface_cell_removes_only_what_is_left_of_it() {
    assert_eq!(row(&[0, 16], 64).dig_fraction([32, 0]), 0.125); // an eighth is left
    assert_eq!(row(&[4, 4], 64).dig_fraction([32, 0]), 1.0);
}

#[test]
fn a_steep_rise_is_a_cliff_and_a_gentle_one_is_not() {
    assert!(row(&[0, 192], 256).is_cliff([40, 0], [41, 0])); // three cells per cell
    assert!(!row(&[0, 64], 128).is_cliff([40, 0], [41, 0])); // one cell per cell
}

#[test]
fn the_surface_is_level_beyond_the_outermost_tile_centres() {
    let g = row(&[0, 64], 128);
    assert_eq!(g.surface_height([2, 0]), 0.0);
    assert_eq!(g.surface_height([116, 0]), 64.0);
}

#[test]
fn ground_at_the_ceiling_is_capped() {
    let g = row(&[8, 80], 64);
    assert!(g.is_capped([120, 0]));
    assert!(!g.is_capped([1, 0]));
}

/// Three tiles in a row at elevation 8 of a terrain with this relief, and these channels.
fn hilly(amplitude: i64, scale: i64, seed: i64, channels: Vec<Channel>) -> Surface {
    let terrain = Terrain {
        id: "HILLY".into(),
        default_elevation: 8,
        relief_amplitude: amplitude,
        relief_scale: scale,
        ..Terrain::default()
    };
    let library = TerrainLibrary { terrains: vec![terrain], ..TerrainLibrary::default() };
    let layout = MapLayout {
        cols: 3,
        rows: 1,
        default_terrain_id: "HILLY".into(),
        seed,
        channels,
        ..MapLayout::default()
    };
    Surface::new(&layout, &library)
}

fn channel(depth: i64, liquid_id: &str, liquid_depth: i64) -> Channel {
    Channel {
        tiles: vec![[0, 0], [1, 0], [2, 0]],
        width: 4,
        depth,
        liquid_id: liquid_id.into(),
        liquid_depth,
    }
}

fn heights(g: &Surface) -> Vec<f64> {
    let mut out = Vec::new();
    for x in (64..128).step_by(2) {
        for y in (0..64).step_by(2) {
            out.push(g.surface_height([x, y]));
        }
    }
    out
}

#[test]
fn no_relief_leaves_the_tile_elevation() {
    assert_eq!(hilly(0, 0, 7, vec![]).surface_height([80, 20]), 8.0);
}

#[test]
fn relief_makes_a_hilly_tile_hilly_within_its_amplitude() {
    let all = heights(&hilly(4, 6, 7, vec![]));
    let (low, high) = all.iter().fold((f64::MAX, f64::MIN), |(l, h), &x| (l.min(x), h.max(x)));
    assert!(high - low > 1.0);
    assert!(low >= 4.0 && high <= 12.0);
}

#[test]
fn relief_is_the_same_for_the_same_seed_and_differs_for_another() {
    let first = heights(&hilly(4, 6, 7, vec![]));
    assert_eq!(heights(&hilly(4, 6, 7, vec![])), first);
    assert_ne!(heights(&hilly(4, 6, 8, vec![])), first);
}

#[test]
fn a_channel_carves_its_depth_along_its_path() {
    let g = hilly(0, 0, 7, vec![channel(3, "", 0)]);
    assert_eq!(g.surface_height([96, 32]), 5.0); // on the line
    assert_eq!(g.surface_height([96, 35]), 8.0); // beyond its width
}

#[test]
fn a_channel_holds_its_liquid_to_a_level() {
    let g = hilly(0, 0, 7, vec![channel(3, "WATER", 2)]);
    assert_eq!(g.liquid_at([96, 32]), Some(("WATER".to_string(), 7.0)));
    assert_eq!(g.liquid_at([96, 36]), None);
}

#[test]
fn a_negative_depth_raises_a_bank() {
    assert_eq!(hilly(0, 0, 7, vec![channel(-2, "", 0)]).surface_height([96, 32]), 10.0);
}

#[test]
fn relief_noise_is_godots_bit_for_bit() {
    // GroundRelief.noise's values in Godot 4.7, read before the GDScript was deleted.
    use super::surface::noise;
    assert_eq!(noise(v(3.0, 5.0), 0, 7), 0.0);
    assert_eq!(noise(v(3.0, 5.0), 6, 7).to_bits(), 0xbfb5ea326c83b3ac);
    assert_eq!(noise(v(100.25, 17.5), 9, -4).to_bits(), 0x3fd1bb5649e2e47d);
    assert_eq!(noise(v(0.5, 63.0), 1, 123456).to_bits(), 0xbfc4c414c414c416);
}
