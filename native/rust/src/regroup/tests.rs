//! ScrumRegroup's scenarios (Decisions 88, 92, 95 and 110), on squads built here: a player
//! squad facing east at the origin, one file of grems a cell a rank, and a kingdom squad
//! further east.

use super::*;
use crate::contest;
use crate::ground::Walker;
use crate::roster::{Listed, Member};
use crate::squads::{Chaser, Stance};
use crate::steer;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules {
        pace: 0.25,
        seconds: 0.1,
        seed: 7,
        reach: 0.2,
        arc: 60.0,
        look: 3.0,
        clear: 0.1,
        trade_seconds: 2.0,
    }
}

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

/// The player squad (slot 0, id 1) of `ranks` grems in one file at their places, facing
/// east from the origin, and a kingdom squad (slot 1, id 2) of `foes` grems standing at
/// `foes_at`, each facing west.
struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

fn built(ranks: usize, foes_at: &[V2]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (slot, (id, faction)) in [(1i64, 0u32), (2, 1)].iter().enumerate() {
        squads.open(*id);
        squads.width[slot] = 1;
        roster.listed.push(Listed {
            squad: slot,
            id: *id,
            faction: *faction,
            living: vec![],
            ..Listed::default()
        });
    }
    for n in 0..ranks {
        columns.grow();
        columns.rank[n] = n as i32;
        columns.speed[n] = 1.0;
        columns.bearing[n] = 90.0;
        let unit = grem(n as i64 + 1, n);
        columns.position[n] = Frame::kept(&squads, 0).place((n as i32, 0), unit.foot);
        roster.listed[0].living.push(unit);
    }
    for (k, &at) in foes_at.iter().enumerate() {
        let slot = columns.position.len();
        columns.grow();
        columns.position[slot] = at;
        columns.bearing[slot] = 270.0;
        columns.speed[slot] = 1.0;
        roster.listed[1].living.push(grem(101 + k as i64, slot));
    }
    squads.state[1] = locks::FIGHTING;
    Built { squads, columns, roster }
}

/// Unit `n` of the player squad goes loose standing at `at`.
fn loosen(b: &mut Built, n: usize, at: V2) {
    let unit = &b.roster.listed[0].living[n];
    entries::loosen(&mut b.squads, &mut b.columns, 0, (unit.id, unit.slot), at);
}

fn regrouper(drilled: bool) -> Regrouper {
    Regrouper { listed: 0, drilled, reform: 1.0, withdrawing: false, homes: vec![] }
}

fn run(b: &mut Built, regroupers: &[Regrouper]) {
    step(&mut b.squads, &mut b.columns, &b.roster, regroupers, None, &rules());
}

fn place(b: &Built, n: usize) -> V2 {
    b.columns.position[b.roster.listed[0].living[n].slot]
}

#[test]
fn a_unit_out_of_its_place_walks_back_towards_it_at_its_reform_pace() {
    let mut b = built(1, &[]);
    let home = place(&b, 0);
    loosen(&mut b, 0, home.add(V2::new(0.0, 2.0)));
    let mut slow = regrouper(false);
    slow.reform = 0.5;
    run(&mut b, &[slow]);
    let at = b.columns.loose_at[0];
    assert!((at.distance_to(home) as f64 - 2.0).abs() > 0.01, "it moved");
    assert!(at.distance_to(home) < 2.0);
    assert!(at.distance_to(home.add(V2::new(0.0, 2.0))) as f64 <= 0.125 + 0.000001);
    assert_eq!(b.columns.next[0], at);
    assert_eq!(b.columns.goal_slot[0], -1);
    assert_eq!(b.squads.loose[0], vec![1]);
}

#[test]
fn a_moving_squad_holds_while_its_units_regroup() {
    let mut b = built(1, &[]);
    let home = place(&b, 0);
    loosen(&mut b, 0, home.add(V2::new(0.0, 1.0)));
    run(&mut b, &[regrouper(false)]);
    assert_eq!(b.squads.state[0], locks::HOLDING);
}

#[test]
fn a_unit_in_its_place_facing_its_way_rejoins_the_frame() {
    let mut b = built(1, &[]);
    let home = place(&b, 0);
    loosen(&mut b, 0, home);
    run(&mut b, &[regrouper(false)]);
    assert!(b.squads.loose[0].is_empty());
}

#[test]
fn a_unit_in_its_place_in_a_stance_stays_loose() {
    let mut b = built(1, &[]);
    let home = place(&b, 0);
    b.squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 90.0, foe: 2 });
    loosen(&mut b, 0, home);
    run(&mut b, &[regrouper(false)]);
    assert_eq!(b.squads.loose[0], vec![1]);
}

#[test]
fn chasers_and_withdrawing_units_are_left_to_their_own_passes() {
    let mut b = built(2, &[]);
    let away = place(&b, 0).add(V2::new(0.0, 2.0));
    loosen(&mut b, 0, away);
    let out = place(&b, 1).add(V2::new(0.0, 2.0));
    loosen(&mut b, 1, out);
    let chaser = Chaser { unit: 1, foe: 2, from: away, leash: 4.0, returning: false };
    b.squads.chase(0, chaser);
    run(&mut b, &[regrouper(false)]);
    assert_eq!(b.columns.loose_at[0], away);
    assert_ne!(b.columns.loose_at[1], out);
    let left = b.columns.loose_at[1];
    let mut withdrawing = regrouper(false);
    withdrawing.withdrawing = true;
    run(&mut b, &[withdrawing]);
    assert_eq!(b.columns.loose_at[1], left);
}

#[test]
fn a_drilled_retreats_unit_backs_away_facing_the_foe_it_touches() {
    let mut b = built(1, &[V2::new(1.0, 0.5)]);
    let home = place(&b, 0);
    loosen(&mut b, 0, V2::new(0.2, 0.5)); // touching the foe, east of its place
    b.columns.bearing[0] = 90.0;
    run(&mut b, &[regrouper(true)]);
    assert!((b.columns.bearing[0] - 90.0).abs() < 0.000001, "still facing the foe");
    assert!(b.columns.loose_at[0].distance_to(home) < V2::new(0.2, 0.5).distance_to(home));
}

#[test]
fn an_undrilled_unit_far_from_its_place_turns_to_walk_back_rather_than_face_the_foe() {
    let mut b = built(1, &[V2::new(1.0, 0.5)]);
    loosen(&mut b, 0, V2::new(0.2, 0.5));
    b.squads.position[0] = V2::new(-20.0, 0.0); // its frame, and its place, far behind it
    run(&mut b, &[regrouper(false)]);
    assert!((b.columns.bearing[0] - 90.0).abs() > 1.0, "it turned for its place");
}

#[test]
fn a_unit_steps_round_a_foreign_body_in_its_way() {
    let mut b = built(1, &[V2::new(-1.5, 0.5)]);
    let home = place(&b, 0);
    loosen(&mut b, 0, home.add(V2::new(-3.0, 0.0)));
    b.squads.state[1] = locks::MOVING;
    run(&mut b, &[regrouper(false)]);
    assert!(b.columns.loose_at[0].y != home.y, "it stepped aside of the body");
}

#[test]
fn the_steering_looks_at_the_near_bodies_as_at_all_of_them() {
    let far: Vec<V2> =
        (0..30).map(|k| V2::new(-1.5 + k as f32 * 0.37, 0.5 + k as f32 * 0.21)).collect();
    let mut b = built(1, &far);
    let home = place(&b, 0);
    loosen(&mut b, 0, home.add(V2::new(-3.0, 0.2)));
    let bodies = b.roster.bodies(&b.squads, &b.columns);
    let crowd = Crowd::of(bodies.clone());
    let unit = &b.roster.listed[0].living[0];
    let at = b.columns.loose_at[0];
    let near = crowd.steer(unit, 1, at, home, &rules().steering());
    let all: Vec<steer::Body> =
        bodies.iter().map(|b| steer::Body { at: b.at, radius: b.radius, squad: b.squad }).collect();
    let r = rules();
    let draw = |i: usize| contest::draw(r.seed, bodies[i].id);
    let own = || contest::draw(r.seed, unit.id);
    let every =
        steer::toward(at, home, unit.radius(), 1, &all, r.look, r.clear, r.seed, &own, &draw);
    assert_eq!(near, every);
}

#[test]
fn the_squads_regroup_the_same_whatever_the_lists_order() {
    let foes = [V2::new(1.0, 0.5), V2::new(1.0, 1.5)];
    let set = |b: &mut Built| {
        loosen(b, 0, V2::new(0.2, 0.5));
        loosen(b, 1, V2::new(0.2, 1.5));
        let out = place(b, 2).add(V2::new(0.0, 1.0));
        loosen(b, 2, out);
    };
    let mut b = built(3, &foes);
    set(&mut b);
    run(&mut b, &[regrouper(true)]);
    let mut r = built(3, &foes);
    set(&mut r);
    r.roster.listed.reverse();
    for listed in &mut r.roster.listed {
        listed.living.reverse();
    }
    let reversed = Regrouper { listed: 1, ..regrouper(true) };
    run(&mut r, &[reversed]);
    assert_eq!(b.columns.loose_at, r.columns.loose_at);
    assert_eq!(b.columns.bearing, r.columns.bearing);
}
