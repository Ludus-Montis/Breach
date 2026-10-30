//! The tick's rules parsed in the crate: each pass's arguments, as `tick::given` builds them,
//! read into the rules `tick::step` takes, with nothing from Godot.

use super::*;
use crate::content::tests::repo_content;
use crate::content::{BattleTuning, Content};
use crate::tick::given::{self, At, Paces};

fn tuning() -> BattleTuning {
    Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}")).tuning
}

fn given(tick: i64) -> Given {
    let at = At { tick, seed: 7, ticks_a_second: 10, rolls: true, threaded: false };
    let paces = Paces { travel_scale: 0.5, seconds: 0.1, route_length: 12.0, cells_a_second: 32.0 };
    given::of(&tuning(), &at, &paces)
}

fn parsed(tick: i64) -> (PathTuning, Rules<'static>) {
    of(&given(tick)).unwrap_or_else(|| panic!("the given rules didn't parse"))
}

#[test]
fn each_pass_reads_the_tick_and_the_seed_it_was_given() {
    let (_, rules) = parsed(30);
    let ticks = [rules.opening.tick, rules.scrum.tick, rules.mustering.tick, rules.ending.tick];
    let seeds = [rules.opening.seed, rules.scrum.seed, rules.mustering.seed, rules.ending.seed];
    assert_eq!((ticks, seeds), ([30; 4], [7; 4]));
    assert_eq!(rules.aftermath.fight.tick, 30);
}

#[test]
fn the_tunings_land_where_the_bindings_put_them() {
    let t = tuning();
    let (climb, rules) = parsed(30);
    assert_eq!(climb.cliff_quarters, t.ground_cliff_quarters);
    assert_eq!(rules.scrum.stall_ticks, (t.scrum_stall_seconds / 0.1).round() as i64);
    assert_eq!(rules.scrum.reach_engage, t.reach_engage);
    assert_eq!(rules.aftermath.seconds, 0.1);
    assert_eq!(rules.aftermath.taking.hold, t.take_hold);
    assert_eq!(rules.ending.wounds_reach, t.wounds_reach);
    assert_eq!(rules.ending.stamina.march_tiring, t.load_march_tiring);
}

#[test]
fn morale_steps_only_on_a_seconds_tick() {
    assert!(parsed(30).1.mustering.morale.is_some());
    assert!(parsed(31).1.mustering.morale.is_none());
}

#[test]
fn a_pass_short_of_its_arguments_parses_to_nothing() {
    let mut short = given(30);
    short.mustering.pop();
    assert!(of(&short).is_none());
}
