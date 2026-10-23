//! Routes and detours (spec 39 step 3b), from the gdUnit tests they replace
//! (test_formation_route, test_route_patch's pure scenarios).

use super::*;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

fn near(a: V2, b: V2) {
    assert!((a.x - b.x).abs() < 0.0001 && (a.y - b.y).abs() < 0.0001, "{a:?} is not {b:?}");
}

/// East 10 cells, then south 6.
fn l_route() -> Route {
    Route::new(vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 6.0)], 0.0)
}

fn straight() -> Route {
    Route::new(vec![v(0.0, 10.0), v(40.0, 10.0)], 3.0)
}

fn patch(from_along: f64, to_along: f64, waypoints: Vec<V2>) -> Patch {
    Patch { from_along, to_along, waypoints }
}

#[test]
fn a_straight_route_runs_east_from_the_origin() {
    let route = Route::new(vec![V2::ZERO, v(64.0, 0.0)], 0.0);
    assert!((route.length() - 64.0).abs() < 0.0001);
    near(route.point_at(16.0), v(16.0, 0.0));
    near(route.heading_at(16.0), v(1.0, 0.0));
}

#[test]
fn points_follow_each_leg_and_clamp_at_the_ends() {
    let route = l_route();
    assert!((route.length() - 16.0).abs() < 0.0001);
    near(route.point_at(4.0), v(4.0, 0.0));
    near(route.point_at(13.0), v(10.0, 3.0));
    assert_eq!(route.point_at(-5.0), v(0.0, 0.0));
    assert_eq!(route.point_at(99.0), v(10.0, 6.0));
}

#[test]
fn the_heading_turns_at_a_bend() {
    let route = l_route();
    near(route.heading_at(9.0), v(1.0, 0.0));
    near(route.heading_at(10.0), v(0.0, 1.0));
}

#[test]
fn a_diagonal_leg_heads_along_it() {
    let route = Route::new(vec![v(0.0, 0.0), v(5.0, 5.0)], 0.0);
    near(route.heading_at(1.0), v(1.0, 1.0).normalized());
}

#[test]
fn an_empty_or_one_point_route_has_no_length_and_heads_east() {
    let empty = Route::new(vec![], 0.0);
    assert_eq!(empty.length(), 0.0);
    assert_eq!(empty.point_at(3.0), V2::ZERO);
    assert_eq!(Route::new(vec![v(2.0, 2.0)], 0.0).heading_at(1.0), v(1.0, 0.0));
}

#[test]
fn a_point_lies_along_the_route_where_it_is_nearest() {
    let route = l_route();
    assert!((route.distance_of(v(4.0, 3.0)) - 4.0).abs() < 0.0001);
    assert!((route.distance_of(v(13.0, 3.0)) - 13.0).abs() < 0.0001);
    assert_eq!(route.distance_of(v(-5.0, -5.0)), 0.0);
}

#[test]
fn a_detour_becomes_a_patch_between_where_it_left_and_rejoined() {
    let detour = [v(10.0, 10.4), v(15.0, 20.0), v(25.0, 20.0), v(30.0, 9.6)];
    let patch = Patch::of_detour(&straight(), &detour);
    assert!((patch.from_along - 10.0).abs() < 0.0001);
    assert!((patch.to_along - 30.0).abs() < 0.0001);
    assert_eq!(patch.waypoints, vec![v(15.0, 20.0), v(25.0, 20.0)]);
}

#[test]
fn the_patched_route_replaces_the_stretch_with_the_detour() {
    let one = patch(10.0, 30.0, vec![v(15.0, 20.0), v(25.0, 20.0)]);
    let patched = Patch::patched(&straight(), &[one]);
    let expected = [(0., 10.), (10., 10.), (15., 20.), (25., 20.), (30., 10.), (40., 10.)];
    assert_eq!(patched.points, expected.map(|(x, y)| v(x, y)).to_vec());
    assert_eq!(patched.half_width, 3.0);
}

#[test]
fn a_patch_over_a_bend_keeps_the_route_either_side() {
    let bent = Route::new(vec![v(0.0, 0.0), v(20.0, 0.0), v(20.0, 20.0)], 0.0);
    let one = patch(15.0, 25.0, vec![v(25.0, 2.0)]);
    let expected = [(0., 0.), (15., 0.), (25., 2.), (20., 5.), (20., 20.)];
    assert_eq!(Patch::patched(&bent, &[one]).points, expected.map(|(x, y)| v(x, y)).to_vec());
}

#[test]
fn patches_apply_in_order_along_the_route_and_overlaps_give_way_whatever_the_list() {
    let later = patch(30.0, 35.0, vec![v(32.0, 5.0)]);
    let first = patch(5.0, 15.0, vec![v(10.0, 15.0)]);
    let overlapping = patch(12.0, 20.0, vec![v(16.0, 0.0)]);
    let expected = [
        (0., 10.),
        (5., 10.),
        (10., 15.),
        (15., 10.),
        (30., 10.),
        (32., 5.),
        (35., 10.),
        (40., 10.),
    ]
    .map(|(x, y)| v(x, y))
    .to_vec();
    let lists = [
        [later.clone(), overlapping.clone(), first.clone()],
        [first.clone(), later.clone(), overlapping.clone()],
        [overlapping, first, later],
    ];
    for list in lists {
        assert_eq!(Patch::patched(&straight(), &list).points, expected);
    }
}

#[test]
fn patches_at_the_same_place_go_by_their_waypoints_whatever_the_list() {
    let west = patch(10.0, 20.0, vec![v(14.0, 2.0)]);
    let east = patch(10.0, 20.0, vec![v(16.0, 2.0)]);
    let one = Patch::patched(&straight(), &[west.clone(), east.clone()]);
    let other = Patch::patched(&straight(), &[east, west]);
    assert_eq!(one, other);
    assert_eq!(one.points[2], v(14.0, 2.0));
}

#[test]
fn a_patch_that_goes_nowhere_is_dropped() {
    let backwards = patch(20.0, 10.0, vec![v(15.0, 0.0)]);
    assert_eq!(Patch::patched(&straight(), &[backwards]).points, straight().points);
}
