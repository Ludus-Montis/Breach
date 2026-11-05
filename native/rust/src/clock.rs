//! The clocks (spec 39 step 9c), ported from GDScript's SimulationClock and SkirmishClock as
//! they were (specs/01-simulation-clock-and-commands.md, specs/21-realtime-skirmish-feel-test.md).
//! They say *when* a tick happens, from the real seconds each frame gives them, and nothing
//! about what a tick does.
//! - `Clock` is the lane's (Decision 1): its elapsed time, scaled by the speed, ticks once
//!   it reaches the tick's length, and is then reset to 0, not carried. Skipping to the next
//!   marker ticks at once, paused or not, and resets it too.
//! - `FixedStep` is the skirmish's: it carries the leftover time between frames, so short
//!   ticks don't drift, and may give several ticks in one frame, at most
//!   `MAX_CATCH_UP_TICKS` (a longer frame's time is dropped).
//!
//! Their settings (the tick's length, the speed) are their bindings' properties, given to
//! each call (`binding.rs`), and the lane's tick is told by its binding. Pure.

mod binding;
#[cfg(test)]
mod tests;

/// The most ticks one frame gives the skirmish.
pub const MAX_CATCH_UP_TICKS: i64 = 20;

/// The skirmish's allowance for a float's rounding when counting whole ticks.
const ROUNDING: f64 = 0.000001;

/// How a clock turns real seconds into ticks: a tick's length, and the speed it plays at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    pub tick_seconds: f64,
    pub speed: f64,
}

/// The lane's clock: the tick number, the time since the last tick, and whether paused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clock {
    tick: i64,
    elapsed: f64,
    paused: bool,
}

impl Clock {
    pub fn tick_number(&self) -> i64 {
        self.tick
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// A frame's `delta` seconds: true when a tick is due, the elapsed time then reset.
    /// Paused, none is due and the time isn't counted.
    pub fn advance_time(&mut self, delta: f64, pace: Pace) -> bool {
        if self.paused {
            return false;
        }
        self.elapsed += delta * pace.speed;
        if self.elapsed < pace.tick_seconds {
            return false;
        }
        self.elapsed = 0.0;
        true
    }

    /// Skipping to the next marker: the elapsed time reset, a tick due at once.
    pub fn skip(&mut self) {
        self.elapsed = 0.0;
    }

    /// A tick counted: its number.
    pub fn advance_tick(&mut self) -> i64 {
        self.tick += 1;
        self.tick
    }
}

/// The skirmish's fixed-step clock: the ticks given, the time carried, and whether paused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FixedStep {
    tick: i64,
    accumulated: f64,
    paused: bool,
}

impl FixedStep {
    pub fn tick_number(&self) -> i64 {
        self.tick
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// A frame's `delta` seconds: how many ticks the caller steps now.
    pub fn advance(&mut self, delta: f64, pace: Pace) -> i64 {
        if self.paused {
            return 0;
        }
        self.accumulated += delta * pace.speed;
        let mut due = ((self.accumulated + ROUNDING) / pace.tick_seconds).floor() as i64;
        if due > MAX_CATCH_UP_TICKS {
            due = MAX_CATCH_UP_TICKS;
            self.accumulated = 0.0;
        } else {
            self.accumulated = (self.accumulated - due as f64 * pace.tick_seconds).max(0.0);
        }
        self.tick += due;
        due
    }

    /// How far into the next tick the time carried is, 0 to 1, for drawing between ticks.
    pub fn fraction(&self, pace: Pace) -> f64 {
        (self.accumulated / pace.tick_seconds).clamp(0.0, 1.0)
    }
}
