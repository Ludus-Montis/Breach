//! Following the command's ways round and looking ahead (spec 39 step 8b3), on one squad
//! built here, its front placed by hand.

use super::*;
use crate::locks::{FIGHTING, HOLDING};
use crate::maths::V2;

/// One squad at `at` (cells), its front `front` tiles along.
fn squad(at: V2, front: f64) -> Squads {
    let mut squads = Squads::default();
    squads.open(1);
    squads.position[0] = at;
    squads.front_distance[0] = front;
    squads
}

/// Ten tiles east along y = 0.
fn lane() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(640.0, 0.0)], 0.0)
}

#[test]
fn a_command_without_a_route_leaves_its_groups_be() {
    assert!(!follows(&squad(V2::new(0.0, 0.0), 0.0), 0, None));
}

#[test]
fn a_group_follows_only_ways_round_it_does_not_carry_yet() {
    let mut squads = squad(V2::new(0.0, 0.0), 0.0);
    assert!(!follows(&squads, 0, Some(0)));
    assert!(follows(&squads, 0, Some(1)));
    squads.patched_count[0] = 1;
    assert!(!follows(&squads, 0, Some(1)));
    assert!(follows(&squads, 0, Some(2)));
}

#[test]
fn a_group_following_stands_where_it_stood_along_the_new_route() {
    let mut squads = squad(V2::new(128.0, 32.0), 9.0);
    let detour = Route::new(vec![V2::new(0.0, 64.0), V2::new(640.0, 64.0)], 0.0);
    follow(&mut squads, 0, &detour, 2);
    assert_eq!(squads.patched_count[0], 2);
    assert_eq!(squads.front_distance[0], 2.0);
}

#[test]
fn a_group_bound_for_the_far_end_keeps_it_as_home() {
    let mut squads = squad(V2::new(64.0, 0.0), 1.0);
    squads.home_distance[0] = 4.0;
    follow(&mut squads, 0, &lane(), 1);
    assert_eq!(squads.home_distance[0], 10.0);
    let mut near = squad(V2::new(64.0, 0.0), 1.0);
    follow(&mut near, 0, &lane(), 1);
    assert_eq!(near.home_distance[0], 0.0);
}

#[test]
fn only_a_marching_group_on_a_route_looks() {
    for state in [HOLDING, FIGHTING] {
        let mut squads = squad(V2::new(0.0, 0.0), 3.0);
        squads.state[0] = state;
        assert!(!looks(&mut squads, 0, true, 32.0));
    }
    let mut squads = squad(V2::new(0.0, 0.0), 3.0);
    assert!(!looks(&mut squads, 0, false, 32.0));
    assert!(looks(&mut squads, 0, true, 32.0));
}

#[test]
fn a_group_looks_again_only_once_its_front_has_moved_on() {
    let mut squads = squad(V2::new(0.0, 0.0), 2.0);
    assert!(looks(&mut squads, 0, true, 32.0));
    assert_eq!(squads.looked_at[0], 128.0);
    squads.front_distance[0] = 2.25;
    assert!(!looks(&mut squads, 0, true, 32.0));
    assert_eq!(squads.looked_at[0], 128.0);
    squads.front_distance[0] = 2.5;
    assert!(looks(&mut squads, 0, true, 32.0));
    assert_eq!(squads.looked_at[0], 160.0);
    squads.front_distance[0] = 2.0;
    assert!(looks(&mut squads, 0, true, 32.0), "back along it counts too");
}
