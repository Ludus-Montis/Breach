//! The tick on a worker (spec 39 step 10d-i), on a stand-in simulation whose tick can be held
//! open: orders queued and applied between ticks, stamped with the tick they were applied at;
//! at most two ticks owed; each tick's events with its snapshot; paused, nothing starts and
//! orders apply at once; a replay's orders at their tick; and the caller never waits on a tick.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use super::*;

/// A simulation whose tick is a count. With a gate, each tick says it has begun and waits to
/// be let through. It says when it is dropped.
struct Counter {
    tick: i64,
    gate: Option<(Sender<()>, Receiver<()>)>,
    dropped: Option<Sender<()>>,
}

impl Steps for Counter {
    /// An order: the tick it was applied at, once applied.
    type Order = Option<i64>;
    /// A tick's events: the tick it made.
    type Told = i64;
    type View = i64;

    fn tick(&self) -> i64 {
        self.tick
    }

    fn apply(&mut self, order: &mut Option<i64>) -> bool {
        *order = Some(self.tick);
        true
    }

    fn step(&mut self) -> i64 {
        if let Some((begun, through)) = &self.gate {
            let _ = begun.send(());
            let _ = through.recv();
        }
        self.tick += 1;
        self.tick
    }

    fn view(&mut self) -> i64 {
        self.tick
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        if let Some(dropped) = &self.dropped {
            let _ = dropped.send(());
        }
    }
}

const PATIENCE: Duration = Duration::from_secs(10);

fn counter() -> Counter {
    Counter { tick: 0, gate: None, dropped: None }
}

/// A counter whose ticks wait at the gate: it, what says a tick has begun, and what lets one
/// through.
fn gated() -> (Counter, Receiver<()>, Sender<()>) {
    let (begun, begins) = mpsc::channel();
    let (through, gate) = mpsc::channel();
    (Counter { tick: 0, gate: Some((begun, gate)), dropped: None }, begins, through)
}

/// The snapshots published until one satisfies `done`, in order, without waiting on the
/// worker: `ready` is asked again until then.
fn until(
    worker: &Worker<Counter>,
    done: impl Fn(&Snapshot<Counter>) -> bool,
) -> Vec<Snapshot<Counter>> {
    let mut seen = Vec::new();
    let began = Instant::now();
    while !seen.iter().any(&done) {
        assert!(began.elapsed() < PATIENCE, "the worker published nothing that satisfies");
        seen.extend(worker.ready());
        std::thread::yield_now();
    }
    seen
}

/// `act` done on another thread; panics if it takes long (it waited on the worker).
fn promptly<T: Send + 'static>(act: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(act());
    });
    finished.recv_timeout(PATIENCE).expect("the caller never waits on the worker")
}

#[test]
fn the_first_snapshot_is_the_state_as_started() {
    let worker = Worker::start(counter());
    let seen = until(&worker, |_| true);
    assert_eq!((seen[0].tick, seen[0].view, seen[0].told), (0, 0, None));
}

#[test]
fn each_ticks_events_travel_with_its_snapshot() {
    let worker = Worker::start(counter());
    let ticked: Vec<(i64, Option<i64>)> =
        worker.run_ticks(3).iter().filter(|s| s.told.is_some()).map(|s| (s.tick, s.told)).collect();
    assert_eq!(ticked, vec![(1, Some(1)), (2, Some(2)), (3, Some(3))]);
}

#[test]
fn an_order_given_during_a_tick_is_applied_after_it_stamped_with_the_tick_applied_at() {
    let (sim, begins, through) = gated();
    let worker = Worker::start(sim);
    assert!(!worker.owe(1));
    begins.recv_timeout(PATIENCE).expect("tick 1 begins");
    worker.order(None);
    let _ = through.send(());
    let seen = until(&worker, |s| !s.taken.is_empty());
    let applied = seen.iter().position(|s| !s.taken.is_empty()).expect("taken");
    let first = seen.iter().position(|s| s.told == Some(1)).expect("tick 1 published");
    assert!(first < applied || seen[applied].told == Some(1), "after tick 1's snapshot");
    assert_eq!(seen[applied].taken, vec![(Some(1), true)], "stamped tick 1, where applied");
}

#[test]
fn the_clock_owes_at_most_two_ticks_and_says_when_time_is_dropped() {
    let worker = Worker::start(counter());
    worker.pause(true);
    assert!(!worker.owe(1));
    assert!(!worker.owe(1));
    assert!(worker.owe(1), "a third tick owed is dropped");
    worker.pause(false);
    let mut seen = until(&worker, |s| s.tick == 2);
    worker.order(None);
    seen.extend(until(&worker, |s| !s.taken.is_empty()));
    assert!(seen.iter().all(|s| s.tick <= 2), "no third tick ran");
    let taken = seen.iter().find(|s| !s.taken.is_empty()).expect("taken");
    assert_eq!(taken.taken, vec![(Some(2), true)]);
}

#[test]
fn paused_no_tick_starts_and_an_order_applies_at_once() {
    let worker = Worker::start(counter());
    worker.pause(true);
    worker.owe(2);
    worker.order(None);
    let seen = until(&worker, |s| !s.taken.is_empty());
    assert!(seen.iter().all(|s| s.told.is_none() && s.tick == 0), "nothing ticked");
    assert_eq!(seen.last().map(|s| s.taken.clone()), Some(vec![(Some(0), true)]));
}

#[test]
fn a_replays_order_is_applied_when_its_tick_comes() {
    let worker = Worker::start(counter());
    worker.order_at(2, None);
    let seen = worker.run_ticks(3);
    let taken = seen.iter().find(|s| !s.taken.is_empty()).expect("the replay's order applied");
    assert_eq!(taken.taken, vec![(Some(2), true)], "between ticks 2 and 3");
    assert_eq!(taken.tick, 3, "published with tick 3, the tick that followed it");
}

#[test]
fn the_caller_never_waits_on_a_tick_and_a_dropped_worker_finishes_it_alone() {
    let (mut sim, begins, through) = gated();
    let (dropped, gone) = mpsc::channel();
    sim.dropped = Some(dropped);
    let worker = Worker::start(sim);
    worker.owe(1);
    begins.recv_timeout(PATIENCE).expect("tick 1 begins");
    let worker = promptly(move || {
        worker.order(None);
        worker.owe(1);
        worker.pause(true);
        let _ = worker.ready();
        worker.pause(false);
        worker
    });
    promptly(move || drop(worker));
    assert!(gone.try_recv().is_err(), "the tick is still running");
    let _ = through.send(());
    gone.recv_timeout(PATIENCE).expect("the old tick finished and the simulation was dropped");
}
