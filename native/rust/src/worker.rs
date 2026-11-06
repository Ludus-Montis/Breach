//! The tick on a worker thread (spec 39 step 10d-i, the owner's answers in #734). The caller
//! (the main thread, which draws) never waits on a tick: it owes ticks as its clock runs,
//! queues orders, pauses and takes what has been published. The worker loops while ticks are
//! owed: at each boundary between ticks it applies the replay's orders that are due, then the
//! queued orders, each stamped with the tick it was applied at (so a record replays them at the
//! same point), then runs one tick and publishes a snapshot holding that tick's events. The
//! clock owes at most `MOST_OWED` ticks; time beyond that is dropped and `owe` says so (the
//! simulation is behind). Paused, no tick starts, and an order is applied as soon as it comes.
//! Dropping the worker never waits: a tick running finishes on its own and is thrown away.
//! `run_ticks` waits, for tests and headless tools, and goes through the same loop; it also
//! waits for every order given before it to be applied (step 10d-ii). `reached` is the tick the
//! field reaches once what is owed is played, so the frame knows whether the newest snapshot
//! is the one its clock is drawing towards.
//!
//! The inbox is a mutex the worker holds only to take work, never across a tick.

use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

#[cfg(test)]
mod tests;

/// The most ticks the clock may owe; time beyond it is dropped.
pub const MOST_OWED: u32 = 2;

/// A simulation the worker can run: its tick, an order applied now, one tick played, and what
/// is drawn of it.
pub trait Steps: Send + 'static {
    /// An order, stamped with the tick it is applied at when applied.
    type Order: Send + 'static;
    /// One tick's events.
    type Told: Send + 'static;
    /// What a snapshot holds to draw.
    type View: Send + 'static;

    /// The ticks played.
    fn tick(&self) -> i64;
    /// `order` given now, stamped with this tick; true if it was taken.
    fn apply(&mut self, order: &mut Self::Order) -> bool;
    /// One tick played; its events.
    fn step(&mut self) -> Self::Told;
    /// What is drawn now.
    fn view(&mut self) -> Self::View;
}

/// The state published after a tick, or after orders were applied between ticks: the ticks
/// played, what is drawn, the events of the tick that made it (none if no tick ran) and the
/// orders applied since the last snapshot, stamped, each with whether it was taken.
pub struct Snapshot<S: Steps> {
    pub tick: i64,
    pub view: S::View,
    pub told: Option<S::Told>,
    pub taken: Vec<(S::Order, bool)>,
    /// The orders given by `order` applied so far, this snapshot's included.
    pub ordered: u64,
}

/// What the caller has handed the worker and it hasn't taken yet.
struct Inbox<O> {
    owed: u32,
    /// Ticks `run_ticks` asked for: run even paused, and not capped.
    forced: u32,
    paused: bool,
    closed: bool,
    /// The tick the field reaches once every tick taken or owed is played.
    reached: i64,
    /// The orders given by `order` so far.
    given: u64,
    orders: Vec<O>,
    /// A replay's orders, each with the tick it is applied at.
    replay: Vec<(i64, O)>,
}

impl<O> Inbox<O> {
    fn idle(&self) -> bool {
        !self.closed
            && self.orders.is_empty()
            && self.replay.is_empty()
            && self.forced == 0
            && (self.owed == 0 || self.paused)
    }
}

struct Shared<O> {
    inbox: Mutex<Inbox<O>>,
    wake: Condvar,
}

/// One unit of work taken from the inbox.
struct Work<O> {
    orders: Vec<O>,
    replay: Vec<(i64, O)>,
    tick: bool,
}

/// A simulation running on its own thread.
pub struct Worker<S: Steps> {
    shared: Arc<Shared<S::Order>>,
    published: Receiver<Snapshot<S>>,
    /// The orders applied, as the snapshots received so far say.
    seen: Cell<u64>,
}

impl<S: Steps> Worker<S> {
    /// `sim` started on a worker thread, which first publishes it as it stands.
    pub fn start(sim: S) -> Worker<S> {
        let inbox = Inbox {
            owed: 0,
            forced: 0,
            paused: false,
            closed: false,
            reached: sim.tick(),
            given: 0,
            orders: Vec::new(),
            replay: Vec::new(),
        };
        let shared = Arc::new(Shared { inbox: Mutex::new(inbox), wake: Condvar::new() });
        let (publish, published) = mpsc::channel();
        let theirs = shared.clone();
        std::thread::spawn(move || run(sim, &theirs, &publish));
        Worker { shared, published, seen: Cell::new(0) }
    }

    /// `ticks` more owed by the clock; true if some were dropped (more than `MOST_OWED`).
    pub fn owe(&self, ticks: u32) -> bool {
        self.change(|inbox| {
            let owed = inbox.owed + ticks;
            inbox.reached += i64::from(owed.min(MOST_OWED) - inbox.owed);
            inbox.owed = owed.min(MOST_OWED);
            owed > MOST_OWED
        })
    }

    /// `order` queued, applied at the next boundary between ticks (at once if paused).
    pub fn order(&self, order: S::Order) {
        self.change(|inbox| {
            inbox.given += 1;
            inbox.orders.push(order);
        });
    }

    /// A replay's `order`, applied at the boundary where `tick` ticks have been played.
    pub fn order_at(&self, tick: i64, order: S::Order) {
        self.change(|inbox| inbox.replay.push((tick, order)));
    }

    pub fn pause(&self, paused: bool) {
        self.change(|inbox| inbox.paused = paused);
    }

    /// The tick the field reaches once every tick taken or owed is played (paused, the owed
    /// ones wait).
    pub fn reached(&self) -> i64 {
        locked(&self.shared.inbox).reached
    }

    /// The snapshots published since last asked, oldest first; never waits for one.
    pub fn ready(&self) -> Vec<Snapshot<S>> {
        let shots: Vec<Snapshot<S>> = self.published.try_iter().collect();
        self.saw(&shots);
        shots
    }

    /// `count` ticks run, paused or not and past the cap, waiting for them and for every
    /// order given before; every snapshot published meanwhile, oldest first.
    pub fn run_ticks(&self, count: u32) -> Vec<Snapshot<S>> {
        let given = self.change(|inbox| {
            inbox.forced += count;
            inbox.reached += i64::from(count);
            inbox.given
        });
        let mut out = Vec::new();
        let mut ticked = 0;
        while ticked < count || self.seen.get() < given {
            let Ok(shot) = self.published.recv() else { break };
            ticked += u32::from(shot.told.is_some());
            self.saw(std::slice::from_ref(&shot));
            out.push(shot);
        }
        out
    }

    /// `shots` received: the orders they applied counted.
    fn saw(&self, shots: &[Snapshot<S>]) {
        if let Some(last) = shots.last() {
            self.seen.set(last.ordered);
        }
    }

    /// `change` made to the inbox, and the worker woken.
    fn change<T>(&self, change: impl FnOnce(&mut Inbox<S::Order>) -> T) -> T {
        let out = change(&mut locked(&self.shared.inbox));
        self.shared.wake.notify_one();
        out
    }
}

impl<S: Steps> Drop for Worker<S> {
    /// Closes the worker without waiting for it: a tick running finishes and is dropped.
    fn drop(&mut self) {
        self.change(|inbox| inbox.closed = true);
    }
}

/// The inbox locked; a worker that panicked leaves it as it was.
fn locked<O>(inbox: &Mutex<Inbox<O>>) -> MutexGuard<'_, Inbox<O>> {
    inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The worker's loop: publish the state as started, then take work until closed.
fn run<S: Steps>(mut sim: S, shared: &Shared<S::Order>, publish: &Sender<Snapshot<S>>) {
    let (tick, view) = (sim.tick(), sim.view());
    let first = Snapshot { tick, view, told: None, taken: Vec::new(), ordered: 0 };
    let _ = publish.send(first);
    let mut replay: Vec<(i64, S::Order)> = Vec::new();
    let mut ordered = 0;
    while let Some(work) = taken(shared) {
        replay.extend(work.replay);
        ordered += work.orders.len() as u64;
        let mut taken = Vec::new();
        replay.sort_by_key(|r| r.0); // stable: a tick's orders in the order given
        let due = replay.partition_point(|r| r.0 <= sim.tick());
        let orders = replay.drain(..due).map(|r| r.1).chain(work.orders);
        for mut order in orders {
            let took = sim.apply(&mut order);
            taken.push((order, took));
        }
        let told = work.tick.then(|| sim.step());
        if told.is_none() && taken.is_empty() {
            continue;
        }
        let shot = Snapshot { tick: sim.tick(), view: sim.view(), told, taken, ordered };
        let _ = publish.send(shot);
    }
}

/// The next work, waiting for some; None once closed.
fn taken<O>(shared: &Shared<O>) -> Option<Work<O>> {
    let mut inbox = locked(&shared.inbox);
    while inbox.idle() {
        inbox = shared.wake.wait(inbox).unwrap_or_else(std::sync::PoisonError::into_inner);
    }
    if inbox.closed {
        return None;
    }
    let tick = if inbox.forced > 0 {
        inbox.forced -= 1;
        true
    } else if inbox.owed > 0 && !inbox.paused {
        inbox.owed -= 1;
        true
    } else {
        false
    };
    let orders = std::mem::take(&mut inbox.orders);
    let replay = std::mem::take(&mut inbox.replay);
    Some(Work { orders, replay, tick })
}
