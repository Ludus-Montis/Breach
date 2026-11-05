//! The clocks as Godot reads them (spec 39 step 9c), each made by `NativeLibrary.made`:
//! - `SimulationClock`, the lane's: main.gd gives it each frame's seconds (`advance_time`), and
//!   the time controls pause, resume, skip and set its properties. A tick is told as
//!   `tick_advanced(tick)` on the object `emit_on` gave (SimEvents), this clock lent while
//!   the handlers run, so they may pause it (a victory does); it pauses after the tick when
//!   `auto_pause_each_tick` is on then.
//! - `SkirmishClock`, the skirmish scenes': `advance` gives the ticks due, `fraction` how far
//!   into the next.
//!
//! Their settings are properties, as GDScript's fields were.

use godot::prelude::*;

use super::{Clock, FixedStep, Pace};

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct SimulationClock {
    #[var]
    #[init(val = 1.0)]
    tick_duration_seconds: f64,
    #[var]
    #[init(val = 1.0)]
    speed_multiplier: f64,
    #[var]
    auto_pause_each_tick: bool,
    clock: Clock,
    /// Where a tick is told (SimEvents); none until `emit_on`.
    events: Option<Gd<Object>>,
    /// Its Godot object (named so: the derive's `base: base` trips newer clippy).
    object: Base<RefCounted>,
}

#[godot_api]
impl SimulationClock {
    /// Each tick told as `tick_advanced(tick)` on `events`.
    #[func]
    fn emit_on(&mut self, events: Gd<Object>) {
        self.events = Some(events);
    }

    #[func]
    fn tick_number(&self) -> i64 {
        self.clock.tick_number()
    }

    #[func]
    fn is_paused(&self) -> bool {
        self.clock.is_paused()
    }

    #[func]
    fn pause(&mut self) {
        self.clock.pause();
    }

    #[func]
    fn resume(&mut self) {
        self.clock.resume();
    }

    /// A frame's `delta` seconds, ticking when a tick is due.
    #[func]
    fn advance_time(&mut self, delta: f64) {
        let pace = Pace { tick_seconds: self.tick_duration_seconds, speed: self.speed_multiplier };
        if self.clock.advance_time(delta, pace) {
            self.advance_tick();
        }
    }

    /// The next marker now, paused or not.
    #[func]
    fn skip_to_next_marker(&mut self) {
        self.clock.skip();
        self.advance_tick();
    }

    /// A tick: counted, told, then paused if `auto_pause_each_tick` is on.
    #[func]
    fn advance_tick(&mut self) {
        let tick = self.clock.advance_tick();
        if let Some(events) = self.events.clone() {
            let mut events = events;
            let _calls_back = self.base_mut();
            events.emit_signal("tick_advanced", &[tick.to_variant()]);
        }
        if self.auto_pause_each_tick {
            self.clock.pause();
        }
    }
}

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct SkirmishClock {
    /// Playtest 1: 0.25 s felt laggy between an order and its effect.
    #[var]
    #[init(val = 0.1)]
    tick_seconds: f64,
    #[var]
    #[init(val = 1.0)]
    speed_multiplier: f64,
    clock: FixedStep,
}

#[godot_api]
impl SkirmishClock {
    /// Real seconds elapsed: how many ticks the caller steps now.
    #[func]
    fn advance(&mut self, delta: f64) -> i64 {
        let pace = self.pace();
        self.clock.advance(delta, pace)
    }

    /// How far into the next tick, 0 to 1, for drawing between ticks.
    #[func]
    fn fraction(&self) -> f64 {
        self.clock.fraction(self.pace())
    }

    #[func]
    fn tick_number(&self) -> i64 {
        self.clock.tick_number()
    }

    #[func]
    fn is_paused(&self) -> bool {
        self.clock.is_paused()
    }

    #[func]
    fn pause(&mut self) {
        self.clock.pause();
    }

    #[func]
    fn resume(&mut self) {
        self.clock.resume();
    }
}

impl SkirmishClock {
    fn pace(&self) -> Pace {
        Pace { tick_seconds: self.tick_seconds, speed: self.speed_multiplier }
    }
}
