//! Steering round bodies (spec 39 step 3b), from the gdUnit tests they replace
//! (test_unit_steer).

use super::*;
use crate::contest::draw;

const SEED: i64 = 4242;
/// BattleTuning's bodies_steer_look and bodies_steer_clear in the scenarios.
const LOOK: f64 = 3.0;
const CLEAR: f64 = 0.05;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

/// A body of another squad's unit (squad 9), half a cell across.
fn body(x: f32, y: f32) -> Body {
    Body { at: v(x, y), radius: 0.5, squad: 9 }
}

/// Unit 1 of squad 1, half a cell across, steering from `at` to `goal`; body `i` is unit
/// `ids[i]`.
fn steer(at: V2, goal: V2, bodies: &[Body], ids: &[i64]) -> V2 {
    let drawn = |i: usize| draw(SEED, ids[i]);
    toward(at, goal, 0.5, 1, bodies, LOOK, CLEAR, SEED, &|| draw(SEED, 1), &drawn)
}

#[test]
fn an_open_way_goes_straight_to_the_goal() {
    let beside = body(3.0, 2.0); // a cell and more clear of the way
    assert_eq!(steer(V2::ZERO, v(6.0, 0.0), &[beside], &[2]), v(6.0, 0.0));
}

#[test]
fn it_steps_round_on_the_side_the_body_leans_away_from() {
    let leans_south = steer(V2::ZERO, v(6.0, 0.0), &[body(2.0, 0.3)], &[2]);
    let leans_north = steer(V2::ZERO, v(6.0, 0.0), &[body(2.0, -0.3)], &[2]);
    assert!(leans_south.y < 0.0); // it passes north of a body south of its way
    assert!(leans_north.y > 0.0);
    assert!(leans_south.x > 0.0);
}

#[test]
fn a_dead_centre_body_is_passed_on_a_seeded_side_whatever_the_list() {
    let one = steer(V2::ZERO, v(6.0, 0.0), &[body(2.0, 0.0), body(2.4, 5.0)], &[2, 3]);
    let other = steer(V2::ZERO, v(6.0, 0.0), &[body(2.4, 5.0), body(2.0, 0.0)], &[3, 2]);
    assert!(one.y.abs() > 0.5);
    assert_eq!(one, other);
}

#[test]
fn bodies_level_along_the_way_go_by_their_draws_whatever_the_list() {
    let (north, south) = (body(2.0, -0.4), body(2.0, 0.4));
    let one = steer(V2::ZERO, v(6.0, 0.0), &[north, south], &[2, 3]);
    let other = steer(V2::ZERO, v(6.0, 0.0), &[south, north], &[3, 2]);
    assert_eq!(one, other);
}

#[test]
fn its_own_squad_and_a_body_on_the_goal_are_left_to_part() {
    let friend = Body { squad: 1, ..body(2.0, 0.0) };
    let on_goal = body(6.0, 0.2);
    assert_eq!(steer(V2::ZERO, v(6.0, 0.0), &[friend], &[2]), v(6.0, 0.0));
    assert_eq!(steer(V2::ZERO, v(6.0, 0.0), &[on_goal], &[3]), v(6.0, 0.0));
}

#[test]
fn walking_on_it_reaches_its_goal_without_running_into_the_body() {
    let foe = body(3.0, 0.1);
    let mut at = V2::ZERO;
    let mut nearest = f32::INFINITY;
    for _ in 0..200 {
        at = at.move_toward(steer(at, v(6.0, 0.0), &[foe], &[2]), 0.1);
        nearest = nearest.min(at.distance_to(foe.at));
    }
    assert!((at.x - 6.0).abs() < 0.0001 && at.y.abs() < 0.0001);
    assert!(nearest >= 1.0 - 0.02); // bodies touch at most, never overlap
}

#[test]
fn rounding_a_body_grazes_it_on_the_side_given() {
    let at = V2::ZERO;
    let centre = v(0.0, -4.0);
    let left = round(at, centre, 1.0, v(-1.0, 0.0));
    let right = round(at, centre, 1.0, v(1.0, 0.0));
    assert!(left.x < 0.0 && right.x > 0.0);
    assert!((left.x + right.x).abs() < 1e-6);
    // already within reach: beside the centre
    assert_eq!(round(at, v(0.0, -0.5), 1.0, v(1.0, 0.0)), v(1.0, -0.5));
}
