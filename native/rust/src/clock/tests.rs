//! The clocks: the former gdUnit scenarios of SimulationClock and SkirmishClock. That a tick
//! is told, and the handlers' calls back while it is, are Godot's (tests/sim/test_clocks.gd).

use super::{Clock, FixedStep, Pace, MAX_CATCH_UP_TICKS};

fn pace(tick_seconds: f64) -> Pace {
    Pace { tick_seconds, speed: 1.0 }
}

/// The lane's clock given a frame, ticking when one is due, as its binding does.
fn frame(clock: &mut Clock, delta: f64, pace: Pace) {
    if clock.advance_time(delta, pace) {
        clock.advance_tick();
    }
}

// SimulationClock

#[test]
fn a_tick_counts_one_more() {
    let mut clock = Clock::default();
    assert_eq!(clock.advance_tick(), 1);
    assert_eq!(clock.tick_number(), 1);
}

#[test]
fn no_tick_before_the_tick_length() {
    let mut clock = Clock::default();
    frame(&mut clock, 1.0, pace(2.0));
    assert_eq!(clock.tick_number(), 0);
}

#[test]
fn a_tick_once_the_tick_length_is_reached() {
    let mut clock = Clock::default();
    frame(&mut clock, 2.0, pace(2.0));
    assert_eq!(clock.tick_number(), 1);
}

#[test]
fn the_elapsed_time_is_reset_on_a_tick_not_carried() {
    let mut clock = Clock::default();
    frame(&mut clock, 3.5, pace(2.0));
    frame(&mut clock, 1.0, pace(2.0));
    assert_eq!(clock.tick_number(), 1);
}

#[test]
fn paused_the_time_neither_ticks_nor_counts() {
    let mut clock = Clock::default();
    clock.pause();
    frame(&mut clock, 10.0, pace(2.0));
    assert_eq!(clock.tick_number(), 0);
    assert!(clock.is_paused());
    clock.resume();
    frame(&mut clock, 1.0, pace(2.0));
    assert_eq!(clock.tick_number(), 0);
}

#[test]
fn resumed_it_ticks_again() {
    let mut clock = Clock::default();
    clock.pause();
    frame(&mut clock, 10.0, pace(2.0));
    clock.resume();
    frame(&mut clock, 2.0, pace(2.0));
    assert_eq!(clock.tick_number(), 1);
    assert!(!clock.is_paused());
}

#[test]
fn skipping_ticks_at_once_and_resets_the_elapsed_time() {
    let mut clock = Clock::default();
    frame(&mut clock, 0.1, pace(2.0));
    clock.skip();
    clock.advance_tick();
    assert_eq!(clock.tick_number(), 1);
    frame(&mut clock, 1.9, pace(2.0));
    assert_eq!(clock.tick_number(), 1);
}

#[test]
fn the_speed_scales_the_time_toward_a_tick() {
    let mut clock = Clock::default();
    frame(&mut clock, 1.0, Pace { tick_seconds: 2.0, speed: 2.0 });
    assert_eq!(clock.tick_number(), 1);
}

// SkirmishClock

#[test]
fn no_skirmish_tick_before_the_tick_length() {
    assert_eq!(FixedStep::default().advance(0.2, pace(0.25)), 0);
}

#[test]
fn the_leftover_time_is_carried_between_frames() {
    let mut clock = FixedStep::default();
    let due = clock.advance(0.3, pace(0.25)) + clock.advance(0.3, pace(0.25));
    assert_eq!(due, 2);
    assert_eq!(clock.tick_number(), 2);
    assert!((clock.fraction(pace(0.25)) - 0.4).abs() < 0.001);
}

#[test]
fn a_tick_just_short_by_rounding_is_given() {
    // 0.3 / 0.1 is 2.9999999999999996 in floats.
    let mut clock = FixedStep::default();
    assert_eq!(clock.advance(0.3, pace(0.1)), 3);
    assert_eq!(clock.fraction(pace(0.1)), 0.0);
}

#[test]
fn paused_the_skirmish_gives_no_ticks_and_resumed_it_goes_on() {
    let mut clock = FixedStep::default();
    clock.pause();
    assert_eq!(clock.advance(5.0, pace(0.25)), 0);
    assert!(clock.is_paused());
    clock.resume();
    assert_eq!(clock.advance(0.25, pace(0.25)), 1);
}

#[test]
fn the_speed_scales_the_skirmish_time() {
    let mut clock = FixedStep::default();
    assert_eq!(clock.advance(0.5, Pace { tick_seconds: 0.25, speed: 2.0 }), 4);
}

#[test]
fn a_long_frame_gives_at_most_the_catch_up_and_drops_the_rest() {
    let mut clock = FixedStep::default();
    assert_eq!(clock.advance(60.0, pace(0.25)), MAX_CATCH_UP_TICKS);
    assert_eq!(clock.fraction(pace(0.25)), 0.0);
    assert_eq!(clock.tick_number(), MAX_CATCH_UP_TICKS);
}

#[test]
fn the_fraction_is_at_most_one() {
    let mut clock = FixedStep::default();
    clock.advance(0.2, pace(0.25));
    assert!((clock.fraction(pace(0.25)) - 0.8).abs() < 1e-9);
    assert_eq!(clock.fraction(pace(0.1)), 1.0);
}
