//! The field as one simulation (spec 39 step 10d-i): played on the worker it is the field
//! `step` plays, and its snapshot copies only the drawn columns that changed.

use std::sync::Arc;

use super::*;
use crate::field_play::tests::content;
use crate::record;
use crate::worker::Worker;

/// A record the feel test played: the via route, then A sent, then A retreated.
const PLAYED: &str = "seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A";
const TICKS: i64 = 300;

fn opened(content: &Arc<Content>) -> FieldSim {
    let seed = record::parse(PLAYED).expect("a record").seed;
    FieldSim::open(content.clone(), (seed, false), "test", Threads::One).expect("the field opens")
}

#[test]
fn the_field_on_the_worker_plays_as_step_plays_it() {
    let content = Arc::new(content());
    let commands = record::parse(PLAYED).expect("a record").commands;
    let mut stepped = opened(&content);
    let mut told = Vec::new();
    for tick in 0..TICKS {
        for given in commands.iter().filter(|c| c.tick == tick) {
            assert!(stepped.given(given), "{} taken", record::line(given));
        }
        told.push(stepped.step());
    }
    let worker = Worker::start(opened(&content));
    for given in &commands {
        worker.order_at(given.tick, given.clone());
    }
    let shots = worker.run_ticks(TICKS as u32);
    let worked: Vec<_> = shots.iter().filter_map(|s| s.told.clone()).collect();
    assert_eq!(worked.len(), told.len());
    assert!(worked == told, "the same events, tick by tick");
    let last = shots.last().expect("a snapshot");
    assert_eq!(last.tick, TICKS);
    assert_eq!(*last.view.positions, stepped.field.columns.position);
    assert_eq!(*last.view.bearings, stepped.field.columns.bearing);
    let taken: Vec<String> =
        shots.iter().flat_map(|s| &s.taken).map(|t| record::line(&t.0)).collect();
    let lines: Vec<String> = commands.iter().map(record::line).collect();
    assert_eq!(taken, lines, "each taken at its tick, stamped with it");
}

#[test]
fn a_snapshot_copies_only_the_drawn_columns_that_changed() {
    let content = Arc::new(content());
    let mut sim = opened(&content);
    let first = sim.view();
    let again = sim.view();
    assert!(Arc::ptr_eq(&first.positions, &again.positions), "nothing changed, nothing copied");
    assert!(Arc::ptr_eq(&first.bearings, &again.bearings));
    assert!(Arc::ptr_eq(&first.ranks, &again.ranks));
    let moved = V2::new(1.0, 2.0);
    sim.field.columns.position[0] = moved;
    let after = sim.view();
    assert!(!Arc::ptr_eq(&again.positions, &after.positions), "the moved column copied");
    assert_eq!(after.positions[0], moved);
    assert!(Arc::ptr_eq(&again.ranks, &after.ranks), "the unmoved columns shared");
    assert!(Arc::ptr_eq(&again.bearings, &after.bearings));
}

#[test]
fn the_snapshot_holds_what_the_scene_draws() {
    let content = Arc::new(content());
    let mut sim = opened(&content);
    let view = sim.view();
    assert_eq!(view.seen, sim.play.seen(&sim.squads, &sim.field, &content));
    let kingdom: Vec<i64> = sim.play.kingdom.iter().map(|&s| sim.squads.id[s]).collect();
    assert_eq!(view.kingdom, kingdom);
    assert_eq!(view.waves, [sim.play.counted(A, &content), sim.play.counted(B, &content)]);
    assert_eq!(view.staging, sim.play.staging_point());
    assert!(Arc::ptr_eq(&sim.content, &content), "the content shared, not copied");
}
