//! The one mover and the ground's toll (spec 39 step 3b), from the gdUnit tests they
//! replace (test_one_mover's steps; test_terrain_toll's through a battle stay gdUnit until
//! their phases port).

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::{GroundTuning, Paint};

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn ground() -> Ground {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    Ground::new(256, 32, GroundTuning::of(&content.tuning))
}

/// A grounded walker a cell tall (TerrainWalker.grounded).
fn walker() -> Walker {
    Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false }
}

/// Facing east, a grem's turn rate and backward pace.
fn east() -> Mover {
    Mover { bearing: 90.0, turn_rate: 450.0, backward_pace: 0.4 }
}

/// Water three cells deep from column 30 east: no grounded walker crosses it.
fn river() -> Ground {
    let mut g = ground();
    g.paint((30, 0, 226, 32), &Paint { depth: Some(3.0), ..Paint::default() });
    g
}

#[test]
fn on_open_ground_the_mover_is_the_units_own_walk() {
    let (at, to) = (v(10.0, 10.0), v(20.0, 12.0));
    let walked = motion::walk(&east(), at, to, 0.7, 0.1, None);
    assert_eq!(step(None, &walker(), &east(), at, to, 0.7, 0.1, None), walked);
    let g = ground();
    assert_eq!(step(Some(&g), &walker(), &east(), at, to, 0.7, 0.1, None), walked);
    assert_eq!(slide(None, &walker(), &east(), at, to, 0.7), motion::advance(&east(), at, to, 0.7));
    assert_eq!(share(Some(&g), &walker(), at, to), 1.0);
}

#[test]
fn a_step_into_ground_it_cannot_cross_is_refused() {
    let g = river();
    let at = v(29.5, 16.5);
    let (moved, _) = step(Some(&g), &walker(), &east(), at, v(40.0, 16.5), 1.0, 0.1, None);
    assert!(moved.x <= 30.0);
    assert!(share(Some(&g), &walker(), at, v(31.0, 16.5)) < 0.000001);
}

#[test]
fn a_step_in_a_wood_goes_at_half_pace() {
    let mut g = ground();
    g.paint((0, 0, 64, 32), &Paint { cost: Some(0.5), ..Paint::default() });
    let at = v(5.5, 5.5);
    assert_eq!(pay(Some(&g), &walker(), at, v(6.5, 5.5)), v(6.0, 5.5));
    assert_eq!(share(Some(&g), &walker(), at, v(6.5, 5.5)), 0.5);
}

#[test]
fn a_barred_way_slides_along_the_barrier_on_the_open_axis_that_brings_it_nearer() {
    let g = river();
    let at = v(29.5, 5.5);
    let (going, open) = way(Some(&g), &walker(), at, v(32.5, 8.5), 1.0);
    assert_eq!(going, v(29.5, 8.5)); // down the bank
    assert_eq!(open, 1.0);
    assert_eq!(pay(Some(&g), &walker(), at, v(30.5, 6.5)), at.move_toward(v(29.5, 6.5), 1.0));
}

#[test]
fn with_no_open_axis_it_stays() {
    let g = river();
    let at = v(29.5, 5.5);
    assert_eq!(way(Some(&g), &walker(), at, v(32.5, 5.5), 1.0), (at, 0.0));
    assert_eq!(pay(Some(&g), &walker(), at, v(30.5, 5.5)), at);
}

#[test]
fn no_ground_is_open_ground() {
    let (at, to) = (v(1.0, 1.0), v(4.0, 5.0));
    assert_eq!(way(None, &walker(), at, to, 1.0), (to, 1.0));
    assert_eq!(pay(None, &walker(), at, to), to);
    assert_eq!(share(None, &walker(), at, to), 1.0);
}
