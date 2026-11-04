//! The command queue: the former gdUnit scenarios of CommandQueue. That a commit is told,
//! and the fill checks as Godot callables, are Godot's (tests/sim/test_lane_queue.gd).

use std::cell::Cell;

use super::Queue;
use crate::clock::Clock;

/// A command and how many of its slots are filled, of how many.
struct Wave<'a> {
    name: &'static str,
    filled: &'a Cell<i64>,
    required: i64,
}

fn wave(filled: &Cell<i64>) -> Wave<'_> {
    Wave { name: "wave of 5", filled, required: 5 }
}

fn names(committed: &[Wave]) -> Vec<&'static str> {
    committed.iter().map(|wave| wave.name).collect()
}

/// A tick's commits, as `LaneState` makes them: the checks, then each taken.
fn commit<'a>(queue: &mut Queue<Wave<'a>>, filled: impl FnMut(&Wave) -> bool) -> Vec<Wave<'a>> {
    let due = queue.due(filled);
    due.into_iter().filter_map(|id| queue.take(id)).collect()
}

fn tick(queue: &mut Queue<Wave<'_>>) -> Vec<&'static str> {
    names(&commit(queue, |wave| wave.filled.get() >= wave.required))
}

#[test]
fn an_enqueued_command_is_pending_by_its_id() {
    let filled = Cell::new(0);
    let mut queue = Queue::default();
    let id = queue.enqueue(wave(&filled));
    assert_eq!(queue.pending(), vec![id]);
    assert_eq!(queue.get(id).map(|wave| wave.name), Some("wave of 5"));
}

#[test]
fn ids_count_from_zero_and_are_never_given_twice() {
    let filled = Cell::new(0);
    let mut queue = Queue::default();
    let first = queue.enqueue(wave(&filled));
    queue.cancel(first);
    let second = queue.enqueue(wave(&filled));
    assert_eq!((first, second), (0, 1));
}

#[test]
fn an_under_filled_command_stays_pending_after_a_tick() {
    let filled = Cell::new(3);
    let mut queue = Queue::default();
    let id = queue.enqueue(wave(&filled));
    assert!(tick(&mut queue).is_empty());
    assert_eq!(queue.pending(), vec![id]);
}

#[test]
fn a_filled_command_waits_for_a_tick() {
    let filled = Cell::new(5);
    let mut queue = Queue::default();
    queue.enqueue(wave(&filled));
    assert_eq!(queue.pending().len(), 1);
}

#[test]
fn a_filled_command_commits_on_a_tick_once() {
    let filled = Cell::new(5);
    let mut queue = Queue::default();
    queue.enqueue(wave(&filled));
    assert_eq!(tick(&mut queue), vec!["wave of 5"]);
    assert!(queue.pending().is_empty());
    assert!(tick(&mut queue).is_empty());
}

#[test]
fn a_cancelled_command_is_gone_and_never_commits() {
    let filled = Cell::new(2);
    let mut queue = Queue::default();
    let id = queue.enqueue(wave(&filled));
    queue.cancel(id);
    assert!(queue.pending().is_empty());
    assert!(queue.get(id).is_none());
    filled.set(5);
    assert!(tick(&mut queue).is_empty());
}

#[test]
fn a_tick_commits_the_filled_in_the_order_enqueued_and_keeps_the_rest() {
    let (full, short) = (Cell::new(5), Cell::new(1));
    let mut queue = Queue::default();
    queue.enqueue(Wave { name: "first", filled: &full, required: 5 });
    let waiting = queue.enqueue(Wave { name: "second", filled: &short, required: 5 });
    queue.enqueue(Wave { name: "third", filled: &full, required: 2 });
    assert_eq!(tick(&mut queue), vec!["first", "third"]);
    assert_eq!(queue.pending(), vec![waiting]);
}

#[test]
fn every_check_is_made_before_any_command_is_taken() {
    let filled = Cell::new(5);
    let mut queue = Queue::default();
    queue.enqueue(wave(&filled));
    queue.enqueue(wave(&filled));
    let checked = Cell::new(0);
    let committed = commit(&mut queue, |_| {
        checked.set(checked.get() + 1);
        true
    });
    assert_eq!((checked.get(), committed.len()), (2, 2));
}

#[test]
fn a_command_cancelled_after_the_checks_is_not_taken() {
    let filled = Cell::new(5);
    let mut queue = Queue::default();
    let first = queue.enqueue(wave(&filled));
    let second = queue.enqueue(wave(&filled));
    assert_eq!(queue.due(|_| true), vec![first, second]);
    queue.cancel(second);
    assert!(queue.take(second).is_none());
    assert_eq!(queue.take(first).map(|wave| wave.name), Some("wave of 5"));
}

#[test]
fn it_commits_only_once_the_slots_fill_and_a_tick_comes() {
    let mut clock = Clock::default();
    let filled = Cell::new(3);
    let mut queue = Queue::default();
    queue.enqueue(wave(&filled));
    clock.advance_tick();
    assert!(tick(&mut queue).is_empty());
    filled.set(5);
    clock.advance_tick();
    assert_eq!(tick(&mut queue), vec!["wave of 5"]);
    assert_eq!(clock.tick_number(), 2);
}
