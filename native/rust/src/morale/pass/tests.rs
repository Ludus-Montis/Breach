//! The morale pass over squads built as `morale/tests.rs` builds them (one unit each, courage
//! 60 from the sheet's default, so a ceiling of 60 with no leader), on the repo's tuning.

use super::*;
use crate::events;
use crate::morale::tests::{tuning, Built};
use crate::morale::RIGHT;

fn bands() -> BandTuning {
    BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 }
}

fn run(b: &mut Built, tick: i64) -> Vec<i64> {
    let mut log = Log::default();
    step((&mut b.squads, &b.columns), &b.roster, (&tuning(), &bands()), tick, &mut log);
    log.take()
}

fn morale(b: &Built, n: usize) -> i32 {
    b.squads.morale[b.roster.listed[n].squad]
}

#[test]
fn a_squad_with_no_morale_yet_starts_at_its_ceiling_and_recovers_no_higher() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.squads.morale[b.roster.listed[squad].squad] = -1;
    assert_eq!(run(&mut b, 10), Vec::<i64>::new());
    assert_eq!(morale(&b, squad), 60);
}

#[test]
fn the_pressure_drains_it_and_a_change_of_band_is_logged() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    b.flanked(squad, RIGHT);
    b.flanked(squad, 2);
    b.squads.morale[b.roster.listed[squad].squad] = 45;
    let s = b.roster.listed[squad].squad;
    let (id, faction) = (b.squads.id[s], i64::from(b.squads.faction[s]));
    let shaken = vec![events::MORALE_BAND as i64, 10, id, faction, bands::WAVERING, 36];
    assert_eq!(run(&mut b, 10), shaken);
    assert_eq!(morale(&b, squad), 36);
}

#[test]
fn a_leader_lifts_the_ceiling_and_the_recovery() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    let slot = b.roster.listed[squad].living[0].slot;
    b.columns.sheet[slot].leadership = 2;
    b.squads.morale[b.roster.listed[squad].squad] = 40;
    run(&mut b, 10);
    assert_eq!(morale(&b, squad), 44);
}

#[test]
fn a_routing_squad_or_one_with_no_one_standing_is_left_as_it_is() {
    let mut b = Built::default();
    let routing = b.squad(1, (10.0, 10.0), 1.0);
    let fallen = b.squad(1, (30.0, 10.0), 1.0);
    b.squads.state[b.roster.listed[routing].squad] = ROUTING;
    b.squads.morale[b.roster.listed[routing].squad] = 0;
    b.roster.listed[fallen].living.clear();
    b.squads.morale[b.roster.listed[fallen].squad] = 20;
    run(&mut b, 10);
    assert_eq!((morale(&b, routing), morale(&b, fallen)), (0, 20));
}
