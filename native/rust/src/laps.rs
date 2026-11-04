//! The tick's laps, for `breach_tick_phases` (spec 39 step 8c-xviii): where a tick's time goes,
//! pass by pass. Each pass of the tick ends a lap where it ends (`lap`), and a lap counts only
//! while `timed` runs on this thread; otherwise `lap` reads one thread-local and returns. A
//! lap times nothing the simulation reads, so it changes no outcome (`phases::tests` holds a
//! timed battle equal to an untimed one).

use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static LAPS: RefCell<Option<Laps>> = const { RefCell::new(None) };
}

/// The laps counted so far: the last one's end, and each phase's microseconds in the order
/// first lapped.
struct Laps {
    last: Instant,
    spent: Vec<(&'static str, f64)>,
}

/// Ends `phase`'s lap: the time since the last lap ended (or `timed` began) is added to it.
pub fn lap(phase: &'static str) {
    LAPS.with(|laps| {
        if let Some(laps) = laps.borrow_mut().as_mut() {
            let now = Instant::now();
            let usec = now.duration_since(laps.last).as_secs_f64() * 1e6;
            match laps.spent.iter_mut().find(|(name, _)| *name == phase) {
                Some((_, spent)) => *spent += usec,
                None => laps.spent.push((phase, usec)),
            }
            laps.last = now;
        }
    });
}

/// Runs `work` with the laps counted, and returns what it returned and each phase's
/// microseconds, in the order first lapped. Time after the last lap is not counted.
pub fn timed<R>(work: impl FnOnce() -> R) -> (R, Vec<(&'static str, f64)>) {
    LAPS.with(|laps| *laps.borrow_mut() = Some(Laps { last: Instant::now(), spent: Vec::new() }));
    let out = work();
    let spent = LAPS.with(|laps| laps.borrow_mut().take()).map_or(Vec::new(), |laps| laps.spent);
    (out, spent)
}
