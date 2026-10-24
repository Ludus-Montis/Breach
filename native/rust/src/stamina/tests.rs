//! FormationStamina's scenarios (spec 28 part 7, Decisions 125 and 126), from
//! test_formation_stamina.gd, on squads built here: unit k (id k + 1) in store slot k, at
//! pace 1 with 100 stamina, in condition 1.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::maths::V2;
use crate::squads::Chaser;

fn tuning() -> StaminaTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    StaminaTuning::of(&content.tuning)
}

fn give_up() -> f64 {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    content.tuning.stamina_give_up
}

fn blow() -> f64 {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    content.tuning.stamina_blow
}

struct Built {
    squads: Squads,
    columns: Columns,
    listed: Vec<Paced>,
}

/// One squad (squad slot n) for each list of stamina, its units in store slots from 0 in
/// the order given, each squad marching.
fn built(staminas: &[&[f64]]) -> Built {
    let mut b = Built { squads: Squads::default(), columns: Columns::default(), listed: vec![] };
    let mut slot = 0;
    for (n, units) in staminas.iter().enumerate() {
        b.squads.open(n as i64 + 1);
        let mut paced = Paced { squad: n, ..Default::default() };
        for &stamina in units.iter() {
            b.columns.grow();
            b.columns.stamina[slot] = stamina;
            b.columns.speed[slot] = 1.0;
            b.columns.condition[slot] = 1.0;
            paced.living.push(Gaited::plain(slot as i64 + 1, slot));
            slot += 1;
        }
        b.listed.push(paced);
    }
    b
}

fn steps(b: &mut Built, ticks: usize, seconds: f64, t: &StaminaTuning) {
    for _ in 0..ticks {
        step(&b.squads, &mut b.columns, &b.listed, seconds, t);
    }
}

#[test]
fn running_tires_it_and_marching_light_costs_nothing() {
    let t = tuning();
    let mut b = built(&[&[100.0], &[100.0], &[100.0]]);
    b.squads.state[0] = locks::ROUTING;
    b.listed.pop(); // the third squad's unit only strikes

    steps(&mut b, 10, 0.1, &t);
    strike(&mut b.columns, &[(2, 1.0)], blow());

    assert!((b.columns.stamina[0] - (100.0 - t.run)).abs() < 0.0001);
    assert_eq!(b.columns.stamina[1], 100.0);
    assert_eq!(b.columns.speed[1], 1.0);
    assert!((b.columns.stamina[2] - (100.0 - blow())).abs() < 0.0001);
}

#[test]
fn a_run_builds_up_and_a_pursuer_reacts_first() {
    let t = tuning();
    let mut b = built(&[&[100.0], &[100.0]]);
    b.squads.state[0] = locks::ROUTING;
    b.listed[1].pursuing = true;

    steps(&mut b, 1, 0.1, &t);
    assert!(b.columns.speed[0] > 1.0); // building up
    assert!(b.columns.speed[0] < 1.5);
    assert_eq!(b.columns.speed[1], 1.0); // still reacting

    steps(&mut b, 30, 0.1, &t);
    assert!((b.columns.speed[0] - 1.5).abs() < 0.0001);
    assert!((b.columns.speed[1] - 1.5).abs() < 0.0001);
}

#[test]
fn a_unit_chasing_on_its_own_runs_and_reacts_first() {
    let t = tuning();
    let mut b = built(&[&[100.0, 100.0]]);
    let chaser = Chaser { unit: 2, foe: 9, from: V2::ZERO, leash: 5.0, returning: false };
    b.squads.chasers[0].push(chaser);

    steps(&mut b, 1, 0.1, &t);
    assert_eq!(b.columns.was_running[0], 0);
    assert_eq!(b.columns.was_running[1], 1);
    assert_eq!(b.columns.speed[1], 1.0); // still reacting
    steps(&mut b, 30, 0.1, &t);
    assert!((b.columns.speed[1] - 1.5).abs() < 0.0001);
    assert_eq!(b.columns.speed[0], 1.0);
}

#[test]
fn it_recovers_only_after_a_breather_from_its_last_cost() {
    let t = tuning();
    let mut b = built(&[&[50.0, 50.0]]);
    b.squads.state[0] = locks::HOLDING;
    b.listed[0].living[1].constitution = 20.0; // half the breather, twice the rate
    let delay = t.delay;

    steps(&mut b, (delay * 0.75 / 0.1).round() as usize, 0.1, &t);
    assert_eq!(b.columns.stamina[0], 50.0); // still catching its breath
    assert!(b.columns.stamina[1] > 50.0);

    steps(&mut b, (delay / 0.1).round() as usize + 10, 0.1, &t);
    assert!(b.columns.stamina[0] > 50.0);
    strike(&mut b.columns, &[(0, 1.0)], blow()); // a blow restarts the breather
    let after = b.columns.stamina[0];
    steps(&mut b, 1, 0.1, &t);
    assert_eq!(b.columns.stamina[0], after);
}

#[test]
fn a_heavy_load_makes_marching_cost_and_a_poor_condition_slows_recovery() {
    let t = tuning();
    let mut b = built(&[&[100.0], &[50.0]]);
    b.listed[0].living[0].load_stage = 2;
    b.squads.state[1] = locks::HOLDING;
    b.columns.condition[1] = 0.0; // too bad to recover

    steps(&mut b, 1, 1.0, &t);
    steps(&mut b, 100, 0.1, &t);

    assert!(b.columns.stamina[0] < 100.0);
    assert_eq!(b.columns.stamina[1], 50.0);
}

#[test]
fn no_breather_on_a_climb_face() {
    let t = tuning();
    let mut b = built(&[&[50.0]]);
    b.squads.state[0] = locks::HOLDING;
    b.columns.on_face[0] = 1;

    steps(&mut b, (t.delay / 0.1).round() as usize + 20, 0.1, &t);

    assert_eq!(b.columns.stamina[0], 50.0);
}

#[test]
fn who_runs() {
    let mut b = built(&[&[100.0], &[100.0], &[100.0], &[100.0, 100.0], &[100.0], &[100.0]]);
    b.listed[0].hurry = true; // hurried
    b.listed[1].hurry = true; // hurried, but fighting
    b.squads.state[1] = locks::FIGHTING;
    b.squads.order[2] = locks::RETREAT; // retreating
    b.squads.order[3] = locks::RETREAT; // withdrawing: its leader keeps a march
    b.listed[3].living[0].leads = true;
    b.listed[3].living[0].retreats_at_march = true;
    b.listed[4].pursuing = true; // following: its units keep a march
    b.listed[4].living[0].pursues_at_march = true;

    let ran: Vec<bool> = b.listed.iter().map(|p| runs(&b.squads, p)).collect();

    assert_eq!(ran, vec![true, false, true, false, false, false]);
}

#[test]
fn a_routing_formation_runs_whatever_its_traits_and_a_returning_pursuit_doesnt() {
    let mut b = built(&[&[100.0], &[100.0]]);
    b.squads.state[0] = locks::ROUTING;
    b.listed[0].living[0].retreats_at_march = true;
    b.listed[1].pursuing = true;
    b.listed[1].returning = true;

    assert!(runs(&b.squads, &b.listed[0]));
    assert!(!runs(&b.squads, &b.listed[1]));
}

#[test]
fn a_spent_unit_cant_run() {
    let t = tuning();
    let mut b = built(&[&[5.0]]);
    b.squads.state[0] = locks::ROUTING;

    steps(&mut b, 1, 0.1, &t);

    assert!((b.columns.speed[0] - t.pace[2]).abs() < 0.0001);
    assert_eq!(b.columns.was_running[0], 0);
}

#[test]
fn tired_it_is_slower_and_spent_slower_still() {
    let t = tuning();
    let mut b = built(&[&[100.0, 40.0, 5.0]]);

    steps(&mut b, 1, 0.0, &t);

    assert_eq!(b.columns.speed[0], 1.0);
    assert!((b.columns.speed[1] - t.pace[1]).abs() < 0.0001);
    assert!((b.columns.speed[2] - t.pace[2]).abs() < 0.0001);
    let stages: Vec<usize> = [100.0, 40.0, 5.0].iter().map(|&s| stage_at(&t, s, 100.0)).collect();
    assert_eq!(stages, vec![0, 1, 2]);
}

#[test]
fn a_mostly_tired_formation_gives_up() {
    let b = built(&[&[100.0, 10.0, 5.0]]);
    let give_up = give_up();

    // one alone giving up is the crate's pursuit's (pursuit/tests.rs)
    assert!(!squad_gives_up(&b.columns, &[(0, 100.0), (1, 100.0)], give_up));
    assert!(squad_gives_up(&b.columns, &[(1, 100.0), (2, 100.0), (0, 100.0)], give_up));
    assert!(!squad_gives_up(&b.columns, &[], give_up));
}

#[test]
fn the_squads_order_changes_nothing() {
    let t = tuning();
    let mut forward = built(&[&[100.0, 30.0], &[60.0, 100.0]]);
    forward.squads.state[0] = locks::ROUTING;
    forward.listed[1].pursuing = true;
    let mut reversed = built(&[&[100.0, 30.0], &[60.0, 100.0]]);
    reversed.squads.state[0] = locks::ROUTING;
    reversed.listed[1].pursuing = true;
    reversed.listed.reverse();
    for paced in &mut reversed.listed {
        paced.living.reverse();
    }

    steps(&mut forward, 20, 0.1, &t);
    steps(&mut reversed, 20, 0.1, &t);

    let (f, r) = (&forward.columns, &reversed.columns);
    assert_eq!(f.stamina, r.stamina);
    assert_eq!(f.speed, r.speed);
    assert_eq!(f.breather, r.breather);
    assert_eq!(f.run_build, r.run_build);
    assert_eq!(f.was_running, r.was_running);
    assert_eq!(f.exerted, r.exerted);
}
