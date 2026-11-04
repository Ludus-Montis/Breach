//! The tick's laps, for `breach_tick_phases` (spec 39 step 8c-xviii): where a tick's time goes,
//! pass by pass. Each pass of the tick ends a lap where it ends (`lap`), and a lap counts only
//! while `timed` runs on this thread; otherwise `lap` reads one thread-local and returns. A
//! lap times nothing the simulation reads, so it changes no outcome (`phases::tests` holds a
//! timed battle equal to an untimed one).
//!
//! A lap also shows the field and the squads to a watcher while `watched` runs on this thread
//! (spec 39 step 8c-xxii: `phases::counts` copies the store's columns there; step 8d:
//! `cuts` fingerprints the whole state there). The watcher only reads them, and its time is
//! left out of the lap that follows.

use std::cell::RefCell;
use std::time::Instant;

use crate::field::Field;
use crate::squads::Squads;

/// What `watched` shows the state to at each lap: the phase that just ended, the field and the
/// squads.
pub type Watcher = Box<dyn FnMut(&'static str, &Field, &Squads)>;

thread_local! {
    static LAPS: RefCell<Option<Laps>> = const { RefCell::new(None) };
    static WATCHER: RefCell<Option<Watcher>> = const { RefCell::new(None) };
}

/// The laps counted so far: the last one's end, and each phase's microseconds in the order
/// first lapped.
struct Laps {
    last: Instant,
    spent: Vec<(&'static str, f64)>,
}

/// Ends `phase`'s lap: the time since the last lap ended (or `timed` began) is added to it,
/// then a watcher, if any, sees `field` and `squads`, and the next lap starts after it.
pub fn lap(phase: &'static str, field: &Field, squads: &Squads) {
    let timing = LAPS.with(|laps| {
        let mut laps = laps.borrow_mut();
        let laps = laps.as_mut()?;
        let now = Instant::now();
        let usec = now.duration_since(laps.last).as_secs_f64() * 1e6;
        match laps.spent.iter_mut().find(|(name, _)| *name == phase) {
            Some((_, spent)) => *spent += usec,
            None => laps.spent.push((phase, usec)),
        }
        laps.last = now;
        Some(())
    });
    let watching = WATCHER.with(|watcher| {
        let mut watcher = watcher.borrow_mut();
        let watch = watcher.as_mut()?;
        watch(phase, field, squads);
        Some(())
    });
    if timing.is_some() && watching.is_some() {
        LAPS.with(|laps| {
            if let Some(laps) = laps.borrow_mut().as_mut() {
                laps.last = Instant::now();
            }
        });
    }
}

/// Runs `work` with the laps counted, and returns what it returned and each phase's
/// microseconds, in the order first lapped. Time after the last lap is not counted.
pub fn timed<R>(work: impl FnOnce() -> R) -> (R, Vec<(&'static str, f64)>) {
    LAPS.with(|laps| *laps.borrow_mut() = Some(Laps { last: Instant::now(), spent: Vec::new() }));
    let out = work();
    let spent = LAPS.with(|laps| laps.borrow_mut().take()).map_or(Vec::new(), |laps| laps.spent);
    (out, spent)
}

/// Runs `work` with `watcher` shown the field and the squads at each lap, and returns what it returned.
pub fn watched<R>(watcher: Watcher, work: impl FnOnce() -> R) -> R {
    WATCHER.with(|slot| *slot.borrow_mut() = Some(watcher));
    let out = work();
    WATCHER.with(|slot| slot.borrow_mut().take());
    out
}
