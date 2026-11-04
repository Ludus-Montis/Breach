//! The lane (Decision 4): the former gdUnit scenarios of LaneSimulation, CombatResolver and
//! PlayerRoster, and the order of a tick's arrivals, which Godot's handlers act between.

use super::{resolve, Blocker, Event, Lane, Unit};
use crate::content::Node;

fn origin() -> Node {
    Node::default()
}

fn fort(hp: i64, dmg: i64) -> Node {
    Node { garrison: 1, garrison_hp: hp, garrison_dmg: dmg, ..Node::default() }
}

fn unit(hp: i64, dmg: i64) -> Unit {
    Unit { hp, dmg }
}

fn blocker(hp: i64, dmg: i64) -> Blocker {
    Blocker { hp, dmg }
}

/// A whole tick with nobody acting between arrivals; its events in order.
fn tick(lane: &mut Lane) -> Vec<Event> {
    let listed = lane.march();
    listed.into_iter().flat_map(|id| lane.arrive(id)).collect()
}

fn captured(events: &[Event]) -> Vec<i64> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Captured { node } => Some(*node),
            Event::Combat { .. } => None,
        })
        .collect()
}

// CombatResolver

#[test]
fn a_horde_destroys_a_blocker_outright_with_no_casualties() {
    let horde = [unit(10, 6), unit(8, 6)];
    let outcome = resolve(&horde, blocker(10, 3));
    assert!(outcome.blocker_destroyed);
    assert_eq!(outcome.remaining_blocker_hp, 0);
    assert_eq!(outcome.surviving_horde, horde.to_vec());
}

#[test]
fn a_surviving_blocker_strikes_the_weakest_first_and_its_overkill_spills() {
    let outcome = resolve(&[unit(10, 2), unit(4, 3), unit(6, 4)], blocker(50, 7));
    assert!(!outcome.blocker_destroyed);
    assert_eq!(outcome.remaining_blocker_hp, 41);
    assert_eq!(outcome.surviving_horde, vec![unit(3, 4), unit(10, 2)]);
}

#[test]
fn a_surviving_blocker_can_wipe_the_whole_horde() {
    let outcome = resolve(&[unit(4, 1), unit(5, 1)], blocker(50, 20));
    assert!(!outcome.blocker_destroyed);
    assert!(outcome.surviving_horde.is_empty());
}

#[test]
fn a_blocker_brought_exactly_to_nothing_is_destroyed() {
    let outcome = resolve(&[unit(10, 5), unit(10, 5)], blocker(10, 9));
    assert!(outcome.blocker_destroyed);
    assert_eq!(outcome.remaining_blocker_hp, 0);
}

#[test]
fn units_of_equal_hp_keep_their_order() {
    let outcome = resolve(&[unit(5, 1), unit(5, 2), unit(3, 3)], blocker(50, 3));
    assert_eq!(outcome.surviving_horde, vec![unit(5, 1), unit(5, 2)]);
}

// LaneSimulation

#[test]
fn a_wave_advances_one_node_a_tick() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    tick(&mut lane);
    assert_eq!(lane.waves()[0].position, 1);
}

#[test]
fn reaching_an_undefended_node_takes_it() {
    let mut lane = Lane::new(&[origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    let events = tick(&mut lane);
    assert_eq!(lane.node(1).0, "player");
    assert_eq!(captured(&events), vec![1]);
}

#[test]
fn a_horde_destroys_a_defended_node_and_takes_it_with_no_casualties() {
    let mut lane = Lane::new(&[origin(), fort(10, 3)]);
    lane.spawn_wave("player", vec![unit(10, 6), unit(8, 6)], 0, 1);
    let events = tick(&mut lane);
    assert_eq!(lane.node(1).0, "player");
    assert_eq!(lane.waves()[0].units.len(), 2);
    assert!(matches!(events[0], Event::Combat { node: 1, mover: None, .. }));
    assert_eq!(captured(&events), vec![1]);
}

#[test]
fn a_horde_that_fails_to_destroy_a_defended_node_takes_casualties_and_not_the_node() {
    let mut lane = Lane::new(&[origin(), fort(50, 7)]);
    lane.spawn_wave("player", vec![unit(4, 1), unit(10, 1)], 0, 1);
    let events = tick(&mut lane);
    assert_eq!(lane.node(1), ("", 48));
    assert_eq!(lane.waves()[0].units, vec![unit(7, 1)]);
    assert!(captured(&events).is_empty());
}

#[test]
fn a_wave_wiped_out_by_a_defended_node_is_gone() {
    let mut lane = Lane::new(&[origin(), fort(50, 20)]);
    lane.spawn_wave("player", vec![unit(4, 1)], 0, 1);
    tick(&mut lane);
    assert!(lane.waves().is_empty());
    tick(&mut lane);
    assert!(lane.waves().is_empty());
}

#[test]
fn a_mid_lane_clash_with_a_moving_blocker_resolves_the_same_way() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 6), unit(8, 6)], 0, 1);
    let mover = lane.spawn_mover("defender", blocker(10, 5), 2, -1);
    let events = tick(&mut lane);
    assert!(lane.movers().is_empty());
    assert_eq!(lane.waves()[0].units.len(), 2);
    assert!(matches!(events[0], Event::Combat { node: 1, mover: Some(m), .. } if m == mover));
}

#[test]
fn a_despawned_wave_is_off_the_lane_and_not_moved() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    let wave = lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    lane.despawn_wave(wave);
    assert!(lane.waves().is_empty());
    tick(&mut lane);
    assert!(lane.waves().is_empty());
}

#[test]
fn a_surviving_moving_blocker_keeps_its_reduced_hp_for_the_next_clash() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 6)], 0, 1);
    lane.spawn_mover("defender", blocker(50, 3), 1, 0);
    tick(&mut lane);
    assert_eq!(lane.movers()[0].blocker.hp, 44);
}

#[test]
fn a_stalemated_wave_does_not_pass_an_undefeated_node() {
    let mut lane = Lane::new(&[origin(), fort(50, 1), origin()]);
    lane.spawn_wave("player", vec![unit(10, 1)], 0, 1);
    tick(&mut lane);
    tick(&mut lane);
    assert_eq!(lane.waves()[0].position, 1);
    assert_eq!(lane.node(1).0, "");
}

#[test]
fn a_stalemated_wave_marches_on_once_it_takes_the_node() {
    let mut lane = Lane::new(&[origin(), fort(2, 1), origin()]);
    lane.spawn_wave("player", vec![unit(10, 1)], 0, 1);
    tick(&mut lane);
    tick(&mut lane);
    assert_eq!(lane.node(1).0, "player");
    tick(&mut lane);
    assert_eq!(lane.waves()[0].position, 2);
}

#[test]
fn a_stalemated_mid_lane_clash_holds_both_sides_in_place() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 1)], 0, 1);
    lane.spawn_mover("defender", blocker(50, 1), 2, -1);
    tick(&mut lane);
    tick(&mut lane);
    assert_eq!(lane.waves()[0].position, 1);
    assert_eq!(lane.movers()[0].position, 1);
}

#[test]
fn a_wave_on_a_node_its_side_owns_does_nothing() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    tick(&mut lane);
    lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    let events = tick(&mut lane);
    assert_eq!(captured(&events), vec![2]);
}

// PlayerRoster

#[test]
fn a_side_s_units_with_none_harvesting_are_those_marching() {
    let mut lane = Lane::new(&[origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2), unit(10, 2)], 0, 1);
    lane.spawn_wave("defender", vec![unit(10, 2)], 1, -1);
    assert_eq!(lane.units_of("player"), 2);
}

#[test]
fn harvesting_units_count_with_none_marching() {
    let mut lane = Lane::new(&[origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2), unit(10, 2), unit(10, 2)], 1, 1);
    lane.harvest(1, "player");
    assert!(lane.waves().is_empty());
    assert_eq!(lane.units_of("player"), 3);
}

#[test]
fn a_side_s_units_are_those_harvesting_and_those_marching() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 2), unit(10, 2)], 1, 1);
    lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    lane.harvest(1, "player");
    assert_eq!(lane.units_of("player"), 3);
}

// The order of a tick's arrivals

#[test]
fn a_wave_despawned_during_a_tick_still_arrives() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    let first = lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    let second = lane.spawn_wave("player", vec![unit(10, 2)], 1, 1);
    lane.spawn_mover("defender", blocker(50, 1), 2, 0);
    let listed = lane.march();
    assert_eq!(listed, vec![first, second]);
    lane.arrive(first);
    lane.despawn_wave(second);
    let events = lane.arrive(second);
    assert!(matches!(events[0], Event::Combat { node: 2, .. }));
    assert_eq!(lane.movers()[0].blocker.hp, 48);
    assert_eq!(lane.waves().len(), 1);
}

#[test]
fn a_mover_spawned_between_arrivals_meets_a_later_wave() {
    let mut lane = Lane::new(&[origin(), origin(), origin()]);
    let first = lane.spawn_wave("player", vec![unit(10, 2)], 0, 1);
    let second = lane.spawn_wave("player", vec![unit(10, 2)], 1, 1);
    lane.march();
    lane.arrive(first);
    lane.spawn_mover("defender", blocker(50, 1), 2, -1);
    let events = lane.arrive(second);
    assert!(matches!(events[0], Event::Combat { node: 2, .. }));
    assert_eq!(lane.node(2).0, "");
}

#[test]
fn a_mover_set_from_outside_fights_with_what_it_was_set() {
    let mut lane = Lane::new(&[origin(), origin()]);
    lane.spawn_wave("player", vec![unit(10, 6)], 0, 1);
    let mover = lane.spawn_mover("defender", blocker(50, 3), 1, 0);
    lane.set_blocker(mover, blocker(20, 3));
    tick(&mut lane);
    assert_eq!(lane.movers()[0].blocker.hp, 14);
}
