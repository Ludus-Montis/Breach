//! The field as presentation plays it (spec 39 step 10d-ii): on the worker, read from its
//! newest snapshot, the frame never waiting; its record kept from the orders taken.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::content::Content;
use crate::field_play::tests::content;
use crate::record;
use crate::threads::Threads;

/// A record the feel test played: the via route, then A sent, then A retreated.
const PLAYED: &str = "seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A";
const TICKS: i64 = 260;
const PATIENCE: Duration = Duration::from_secs(20);

fn opened(content: &Arc<Content>) -> FieldSim {
    let seed = record::parse(PLAYED).expect("a record").seed;
    FieldSim::open(content.clone(), (seed, false), "test", Threads::One).expect("the field opens")
}

/// `PLAYED` stepped in place: the field, and each tick's events.
fn stepped(content: &Arc<Content>) -> (FieldSim, Vec<FieldTold>) {
    let commands = record::parse(PLAYED).expect("a record").commands;
    let mut sim = opened(content);
    let mut told = Vec::new();
    for tick in 0..TICKS {
        for given in commands.iter().filter(|c| c.tick == tick) {
            sim.given(given);
        }
        told.push(sim.step());
    }
    (sim, told)
}

#[test]
fn orders_given_between_ticks_play_and_record_as_step_does() {
    let content = Arc::new(content());
    let (sim, told) = stepped(&content);
    let commands = record::parse(PLAYED).expect("a record").commands;
    let mut player = FieldPlayer::start(opened(&content));
    let mut played = Vec::new();
    for tick in 0..TICKS {
        for given in commands.iter().filter(|c| c.tick == tick) {
            player.order(given.clone());
        }
        played.extend(player.run_ticks(1));
    }
    assert!(played == told, "the same events, tick by tick");
    assert_eq!(player.tick, TICKS);
    assert_eq!(player.record, sim.record, "the record kept from the orders taken");
    assert_eq!(*player.newest.positions, sim.field.columns.position);
}

#[test]
fn a_replay_given_at_the_start_plays_its_orders_at_their_ticks() {
    let content = Arc::new(content());
    let (sim, _) = stepped(&content);
    let mut player = FieldPlayer::start(opened(&content));
    player.replay(record::parse(PLAYED).expect("a record").commands);
    let _ = player.run_ticks(TICKS as u32);
    assert_eq!(player.record, sim.record);
    assert_eq!(player.newest.seen, sim.play.seen(&sim.squads, &sim.field, &content));
}

#[test]
fn the_frame_takes_what_is_ready_without_waiting_and_loses_no_events() {
    let content = Arc::new(content());
    let (_, told) = stepped(&content);
    let mut player = FieldPlayer::start(opened(&content));
    assert_eq!(player.tick, 0, "the field as opened, before the worker publishes");
    assert!(player.caught_up());
    let mut taken = Vec::new();
    let began = Instant::now();
    while player.tick < 4 {
        assert!(began.elapsed() < PATIENCE, "the worker played the ticks owed");
        if player.caught_up() {
            assert!(!player.owe(1), "one tick at a time drops nothing");
            assert!(!player.caught_up(), "owed, and not yet played");
        }
        taken.extend(player.take());
        std::thread::yield_now();
    }
    assert!(taken == told[..4], "every tick's events, oldest first");
}

#[test]
fn an_order_is_recorded_once_the_worker_takes_it() {
    let content = Arc::new(content());
    let mut player = FieldPlayer::start(opened(&content));
    let _ = player.run_ticks(5);
    player.order(record::from_words(0, "send A").expect("words"));
    let _ = player.run_ticks(0);
    assert_eq!(player.record.last().map(String::as_str), Some("5 player send A"));
}
