//! The commands' table (spec 39 step 8b4-iii).

use super::*;

fn v(x: f32, y: f32) -> V2 {
    V2::new(x, y)
}

#[test]
fn a_new_command_has_no_route_and_pursues_and_takes() {
    let mut commands = Commands::default();
    let first = commands.open(2.0);
    let second = commands.open(5.0);
    assert_eq!((first, second), (0, 1));
    assert_eq!(commands.home, vec![2.0, 5.0]);
    assert_eq!(commands.route[1], None);
    assert!(commands.gives(1, PURSUES) && commands.gives(1, TAKES));
    assert!(!commands.gives(1, HURRY) && !commands.gives(1, MERGES));
    assert_eq!(commands.tends[1], "");
    assert!(commands.patches[1].is_empty());
}

#[test]
fn an_order_given_and_taken_back() {
    let mut commands = Commands::default();
    commands.open(0.0);
    commands.give(0, HURRY, true);
    commands.give(0, PURSUES, false);
    assert!(commands.gives(0, HURRY) && !commands.gives(0, PURSUES) && commands.gives(0, TAKES));
}

#[test]
fn a_way_round_is_kept_as_a_patch_in_the_order_learned() {
    let mut commands = Commands::default();
    commands.open(0.0);
    let lane = Route::new(vec![v(0.0, 0.0), v(640.0, 0.0)], 0.0);
    commands.route[0] = Some(lane.clone());
    let detour = [v(64.0, 0.0), v(96.0, 32.0), v(128.0, 0.0)];
    commands.learn(0, &detour);
    commands.learn(0, &[v(320.0, 0.0), v(352.0, -32.0), v(384.0, 0.0)]);
    assert_eq!(commands.patches[0].len(), 2);
    assert_eq!(commands.patches[0][0], Patch::of_detour(&lane, &detour));
}
