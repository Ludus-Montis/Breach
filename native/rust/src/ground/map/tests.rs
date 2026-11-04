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
fn only_the_field_is_a_map() {
    let tuning = tuning();
    assert!(of("field", &tuning).is_some());
    assert!(of("nowhere", &tuning).is_none());
}
