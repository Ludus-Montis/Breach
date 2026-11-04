//! FormationField's ground made from its map (spec 39 step 8c-v), as
//! test_formation_field's ground scenarios read it.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::Walker;
use crate::maths::V2;

fn tuning() -> BattleTuning {
    Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}")).tuning
}

fn made() -> Ground {
    let tuning = tuning();
    field(&tuning).ground(GroundTuning::of(&tuning))
}

/// The centre of cell (x, y).
fn at(x: i32, y: i32) -> V2 {
    V2::new(x as f32 + 0.5, y as f32 + 0.5)
}

/// A grem's height in cells, with no swimming or climbing of its own.
fn grem() -> Walker {
    Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false }
}

#[test]
fn the_field_is_two_tiles_by_one() {
    assert_eq!(made().size(), (128, 64));
}

#[test]
fn the_wood_is_half_pace_hides_and_obscures() {
    let ground = made();
    let tuning = tuning();
    assert_eq!(ground.crossing(&grem(), at(18, 10), at(19, 10)), 0.5);
    assert!(ground.blocks_sight(at(18, 10)));
    assert_eq!(ground.obscurance_at(at(18, 10)), tuning.sight_wood_obscurance as f32 as f64);
    assert_eq!(ground.crossing(&grem(), at(18, 30), at(19, 30)), 1.0);
    assert!(!ground.blocks_sight(at(18, 30)));
}

#[test]
fn the_stream_is_too_deep_for_a_grem_but_the_ford_is_not() {
    let ground = made();
    assert_eq!(ground.crossing(&grem(), at(27, 10), at(28, 10)), 0.0);
    assert!(ground.crossing(&grem(), at(27, 20), at(28, 20)) > 0.0);
}

#[test]
fn the_hill_rises_a_quarter_a_ring_to_two_cells() {
    let ground = made();
    assert_eq!(ground.height_at(at(92, 16)), 1);
    assert_eq!(ground.height_at(at(93, 17)), 2);
    assert_eq!(ground.height_at(at(101, 31)), 8);
    assert_eq!(ground.height_at(at(91, 31)), 0);
}

#[test]
fn only_the_field_and_the_mixed_ground_are_maps() {
    let tuning = tuning();
    assert!(of("field", &tuning).is_some());
    assert!(of("mixed", &tuning).is_some());
    assert!(of("nowhere", &tuning).is_none());
}

fn mixed_ground() -> Ground {
    let tuning = tuning();
    mixed(&tuning).ground(GroundTuning::of(&tuning))
}

#[test]
fn the_mixed_ground_s_wood_crosses_the_line_of_advance_at_half_pace() {
    let ground = mixed_ground();
    assert_eq!(ground.size(), (128, 64));
    for y in [0, 32, 63] {
        assert_eq!(ground.crossing(&grem(), at(31, y), at(32, y)), 0.5, "into the wood at y {y}");
        assert!(ground.blocks_sight(at(40, y)));
        assert_eq!(ground.crossing(&grem(), at(47, y), at(48, y)), 1.0, "out of it at y {y}");
    }
}

#[test]
fn the_mixed_ground_s_stream_is_too_deep_for_a_grem_but_its_ford_is_not() {
    let ground = mixed_ground();
    for y in [0, 15, 48, 63] {
        assert_eq!(ground.crossing(&grem(), at(79, y), at(80, y)), 0.0, "the stream at y {y}");
    }
    for y in [16, 32, 47] {
        assert!(ground.crossing(&grem(), at(79, y), at(80, y)) > 0.0, "the ford at y {y}");
    }
}
