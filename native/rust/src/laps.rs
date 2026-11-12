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
//!
//! A phase's own parts are laps too (spec 41 step 1): each part but the last ends with
//! `split`, which counts its time but shows the watcher nothing, and the last ends the phase
//! with `lap`. So the watcher still sees the state once a phase, as the phase ends, and the
//! counts and the cuts stay a phase's: a split's work shows in the lap that ends its phase.
//!
//! Beside the wall clock, each lap reads the process's CPU time (spec 50 step 3), where the
//! system gives a fine one (`cpu`): a lap's CPU time over its wall time is how many threads
//! were busy in it, on average. Rayon's idle workers spin a little before they sleep, so a
//! short phase right after a threaded one can read a little over 1 with no work threaded.

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

/// The laps counted so far: the last one's end on the wall clock and the CPU clock, and each
/// phase's microseconds on both, in the order first lapped.
struct Laps {
    last: Instant,
    last_cpu: Option<f64>,
    spent: Vec<(&'static str, f64)>,
    cpu: Vec<(&'static str, f64)>,
}

/// What `clocked` counted: each phase's wall microseconds in the order first lapped, and its
/// CPU microseconds in the same order, or none where the system has no fine CPU clock.
#[derive(Debug, Default)]
pub struct Lapped {
    pub wall: Vec<(&'static str, f64)>,
    pub cpu: Option<Vec<(&'static str, f64)>>,
}

/// The process's CPU time so far, in microseconds, summed over its threads; none on a system
/// without a fine clock for it (Windows' updates every ~15.6 ms, too coarse for most phases).
#[cfg(unix)]
fn cpu() -> Option<f64> {
    let mut at = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `at` is a valid timespec for the call to write; nothing else is read.
    let read = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut at) };
    (read == 0).then(|| at.tv_sec as f64 * 1e6 + at.tv_nsec as f64 / 1e3)
}

#[cfg(not(unix))]
fn cpu() -> Option<f64> {
    None
}

/// Adds `usec` to `part`'s, or lists `part` with it.
fn add(spent: &mut Vec<(&'static str, f64)>, part: &'static str, usec: f64) {
    match spent.iter_mut().find(|(name, _)| *name == part) {
        Some((_, total)) => *total += usec,
        None => spent.push((part, usec)),
    }
}

/// Ends `phase`'s lap: the time since the last lap ended (or `timed` began) is added to it,
/// then a watcher, if any, sees `field` and `squads`, and the next lap starts after it.
pub fn lap(phase: &'static str, field: &Field, squads: &Squads) {
    let timing = counted(phase);
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
                laps.last_cpu = cpu();
            }
        });
    }
}

/// Ends `part`'s lap without showing the watcher the state: the time since the last lap
/// ended is added to it.
pub fn split(part: &'static str) {
    counted(part);
}

/// Adds the time since the last lap ended to `part`'s, if laps are counted on this thread.
fn counted(part: &'static str) -> Option<()> {
    LAPS.with(|laps| {
        let mut laps = laps.borrow_mut();
        let laps = laps.as_mut()?;
        let (now, now_cpu) = (Instant::now(), cpu());
        add(&mut laps.spent, part, now.duration_since(laps.last).as_secs_f64() * 1e6);
        if let (Some(now_cpu), Some(last_cpu)) = (now_cpu, laps.last_cpu) {
            add(&mut laps.cpu, part, now_cpu - last_cpu);
        }
        laps.last = now;
        laps.last_cpu = now_cpu;
        Some(())
    })
}

/// Runs `work` with the laps counted, and returns what it returned and each phase's
/// microseconds, in the order first lapped. Time after the last lap is not counted.
pub fn timed<R>(work: impl FnOnce() -> R) -> (R, Vec<(&'static str, f64)>) {
    let (out, lapped) = clocked(work);
    (out, lapped.wall)
}

/// `timed`, with each phase's CPU microseconds beside its wall ones (`Lapped`).
pub fn clocked<R>(work: impl FnOnce() -> R) -> (R, Lapped) {
    let started = (Instant::now(), cpu());
    LAPS.with(|laps| {
        *laps.borrow_mut() =
            Some(Laps { last: started.0, last_cpu: started.1, spent: Vec::new(), cpu: Vec::new() })
    });
    let out = work();
    let lapped = LAPS.with(|laps| laps.borrow_mut().take()).map_or(Lapped::default(), |laps| {
        let cpu = laps.last_cpu.map(|_| laps.cpu);
        Lapped { wall: laps.spent, cpu }
    });
    (out, lapped)
}

/// Runs `work` with `watcher` shown the field and the squads at each lap, and returns what it returned.
pub fn watched<R>(watcher: Watcher, work: impl FnOnce() -> R) -> R {
    WATCHER.with(|slot| *slot.borrow_mut() = Some(watcher));
    let out = work();
    WATCHER.with(|slot| slot.borrow_mut().take());
    out
}

#[cfg(test)]
mod tests;
