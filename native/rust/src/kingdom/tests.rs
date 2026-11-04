//! The kingdom's response: the former gdUnit scenarios of SuspicionSystem, TaskForceDispatch,
//! ScriptedBeatWatcher and their two integration replays (specs/00's beats), and main.gd's
//! rule for a task force consumed.

use std::collections::BTreeMap;

use super::{Dispatch, Suspicion, Tier, Watcher, DEFENDER, PLAYER};
use crate::content::Node;
use crate::lane::{Blocker, Event, Lane, Unit};

/// Wary 25, Alarmed 50, Mobilized 75, Full Alert 90.
fn suspicion(decay_per_tick: i64) -> Suspicion {
    Suspicion::new(&[25.0, 50.0, 75.0, 90.0], decay_per_tick)
}

fn hero_party() -> Blocker {
    Blocker { hp: 70, dmg: 18 }
}

fn mobilized_sends(blocker: Blocker) -> BTreeMap<i64, Blocker> {
    BTreeMap::from([(Tier::Mobilized.number(), blocker)])
}

fn origins(count: usize) -> Lane {
    Lane::new(&vec![Node::default(); count])
}

/// A whole tick with nobody acting between arrivals; its events in order.
fn tick(lane: &mut Lane) -> Vec<Event> {
    let listed = lane.march();
    listed.into_iter().flat_map(|id| lane.arrive(id)).collect()
}

// SuspicionSystem

#[test]
fn starts_calm_at_zero() {
    let suspicion = suspicion(0);
    assert_eq!(suspicion.meter(), 0);
    assert_eq!(suspicion.tier(), Tier::Calm);
}

#[test]
fn adding_suspicion_raises_the_meter() {
    let mut suspicion = suspicion(0);
    suspicion.add(10.0);
    assert_eq!(suspicion.meter(), 10);
}

#[test]
fn crossing_a_threshold_changes_the_tier_and_tells_it() {
    let mut suspicion = suspicion(0);
    assert_eq!(suspicion.add(80.0), Some(Tier::Mobilized));
    assert_eq!(suspicion.tier(), Tier::Mobilized);
}

#[test]
fn staying_in_a_tier_tells_nothing() {
    let mut suspicion = suspicion(0);
    suspicion.add(30.0);
    assert_eq!(suspicion.add(5.0), None);
    assert_eq!(suspicion.tier(), Tier::Wary);
}

#[test]
fn the_meter_is_clamped_to_one_hundred() {
    let mut suspicion = suspicion(0);
    suspicion.add(500.0);
    assert_eq!(suspicion.meter(), 100);
    assert_eq!(suspicion.tier(), Tier::FullAlert);
}

#[test]
fn decay_lowers_the_meter_each_tick() {
    let mut suspicion = suspicion(5);
    suspicion.add(20.0);
    suspicion.decay();
    assert_eq!(suspicion.meter(), 15);
}

#[test]
fn decay_can_move_the_tier_back_down_and_tells_it() {
    let mut suspicion = suspicion(30);
    suspicion.add(30.0);
    assert_eq!(suspicion.tier(), Tier::Wary);
    assert_eq!(suspicion.decay(), Some(Tier::Calm));
    assert_eq!(suspicion.tier(), Tier::Calm);
}

#[test]
fn the_meter_does_not_decay_below_zero() {
    let mut suspicion = suspicion(100);
    suspicion.add(10.0);
    suspicion.decay();
    assert_eq!(suspicion.meter(), 0);
}

#[test]
fn the_meter_drops_its_fraction() {
    let mut suspicion = suspicion(0);
    suspicion.add(24.9);
    assert_eq!(suspicion.meter(), 24);
    assert_eq!(suspicion.tier(), Tier::Calm);
}

#[test]
fn a_threshold_not_given_is_never_reached() {
    let mut suspicion = Suspicion::new(&[25.0, 50.0], 0);
    assert_eq!(suspicion.add(100.0), Some(Tier::Alarmed));
}

// TaskForceDispatch

#[test]
fn a_tier_with_no_unit_sends_nothing() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, BTreeMap::new());
    assert_eq!(dispatch.entered(Tier::Wary, &mut lane), None);
    assert!(lane.movers().is_empty());
}

#[test]
fn a_tier_with_a_unit_spawns_the_defenders_mover_on_the_lane() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    let id = dispatch.entered(Tier::Mobilized, &mut lane);
    let movers = lane.movers();
    assert_eq!(movers.len(), 1);
    assert_eq!(id, Some(movers[0].id));
    assert_eq!(movers[0].owner, DEFENDER);
    assert_eq!((movers[0].position, movers[0].direction), (3, -1));
    assert_eq!(movers[0].blocker, hero_party());
    assert_eq!(dispatch.dispatched(), &[movers[0].id]);
}

#[test]
fn a_completed_task_force_is_no_longer_dispatched() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    let id = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    dispatch.completed(id);
    assert!(dispatch.dispatched().is_empty());
}

#[test]
fn a_consumed_task_force_is_no_longer_dispatched() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    let id = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    dispatch.consumed(id);
    assert!(dispatch.dispatched().is_empty());
}

#[test]
fn only_the_task_force_named_is_let_go() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    let first = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    let second = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    dispatch.consumed(second);
    assert_eq!(dispatch.dispatched(), &[first]);
}

#[test]
fn a_mover_lost_in_a_tick_consumes_the_first_dispatched() {
    let mut lane = origins(4);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    let first = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    let second = dispatch.entered(Tier::Mobilized, &mut lane).unwrap();
    dispatch.after_tick(2, 2);
    assert_eq!(dispatch.dispatched(), &[first, second]);
    dispatch.after_tick(2, 1);
    assert_eq!(dispatch.dispatched(), &[second]);
}

#[test]
fn a_mover_lost_with_none_dispatched_consumes_nothing() {
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));
    dispatch.after_tick(1, 0);
    assert!(dispatch.dispatched().is_empty());
}

// ScriptedBeatWatcher

#[test]
fn reaching_the_core_wins() {
    let mut watcher = Watcher::new(3);
    assert!(watcher.captured(3));
}

#[test]
fn victory_is_told_once_though_the_core_is_captured_again() {
    let mut watcher = Watcher::new(3);
    watcher.captured(3);
    assert!(!watcher.captured(3));
}

#[test]
fn capturing_another_node_does_not_win() {
    let mut watcher = Watcher::new(3);
    assert!(!watcher.captured(1));
}

#[test]
fn losing_the_whole_force_loses() {
    let mut watcher = Watcher::new(3);
    assert!(watcher.counted(0));
}

#[test]
fn units_left_do_not_lose() {
    let mut watcher = Watcher::new(3);
    assert!(!watcher.counted(3));
}

#[test]
fn defeat_is_told_once_though_counted_at_zero_again() {
    let mut watcher = Watcher::new(3);
    watcher.counted(0);
    assert!(!watcher.counted(0));
    assert!(!watcher.counted(0));
}

// The integration replays (specs/00's beats)

#[test]
fn a_spike_to_mobilized_dispatches_the_hero_party_and_its_defeat_consumes_it() {
    let mut lane = origins(4);
    let mut suspicion = suspicion(0);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(hero_party()));

    // Beat 3: attacking the fort raises suspicion by a detection spike.
    let tier = suspicion.add(80.0).unwrap();
    dispatch.entered(tier, &mut lane);
    assert_eq!(dispatch.dispatched().len(), 1);

    // Beat 5: the player's horde (80 dmg) destroys the 70-hp Hero Party where they meet.
    lane.spawn_wave(PLAYER, vec![Unit { hp: 20, dmg: 40 }, Unit { hp: 20, dmg: 40 }], 1, 1);
    let before = lane.movers().len();
    tick(&mut lane);
    dispatch.after_tick(before, lane.movers().len());

    assert!(lane.movers().is_empty());
    assert!(dispatch.dispatched().is_empty());
}

#[test]
fn the_fort_then_the_hero_party_then_the_core_wins() {
    let fort = Node { garrison: 1, garrison_hp: 10, garrison_dmg: 3, ..Node::default() };
    let mut lane = Lane::new(&[Node::default(), fort, Node::default(), Node::default()]);
    let mut suspicion = suspicion(0);
    let mut dispatch = Dispatch::new(3, -1, mobilized_sends(Blocker { hp: 20, dmg: 5 }));
    let mut watcher = Watcher::new(3);
    let mut won = false;
    let mut watch = |events: &[Event], watcher: &mut Watcher| {
        for event in events {
            if let Event::Captured { node } = event {
                won |= watcher.captured(*node);
            }
        }
        won
    };

    // Beat 2: the horde destroys the fort outright (20 dmg > 10 hp) and takes it.
    lane.spawn_wave(PLAYER, vec![Unit { hp: 30, dmg: 20 }], 0, 1);
    let events = tick(&mut lane);
    assert!(!watch(&events, &mut watcher));
    assert_eq!(lane.node(1).0, PLAYER);

    // Beat 3: the spike crosses into Mobilized and the Hero Party leaves the Core.
    let tier = suspicion.add(80.0).unwrap();
    dispatch.entered(tier, &mut lane);

    // Beats 4 and 5: they meet at the midpoint and the horde wins outright.
    let before = lane.movers().len();
    let events = tick(&mut lane);
    dispatch.after_tick(before, lane.movers().len());
    assert!(!watch(&events, &mut watcher));
    assert!(lane.movers().is_empty());
    assert!(dispatch.dispatched().is_empty());
    assert!(!lane.waves()[0].units.is_empty());

    // The closing beat: the horde reaches the Core and wins.
    let events = tick(&mut lane);
    assert!(watch(&events, &mut watcher));
}
