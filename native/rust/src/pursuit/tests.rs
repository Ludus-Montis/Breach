//! The pursuits' scenarios (Decisions 95, 103, 109 and 112), on squads built here: a player
//! squad facing east at the origin, one file of grems a cell a rank, and a kingdom squad
//! retreating to the east.

use super::*;
use crate::ground::Walker;
use crate::locks;
use crate::roster::Listed;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules {
        cells_per_second: 2.5,
        seconds: 0.1,
        give_up: 0.2,
        steering: Steering { look: 3.0, clear: 0.1, seed: 7 },
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

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

/// The player squad (slot 0, id 1) of `ranks` grems at their places, and the kingdom squad
/// (slot 1, id 2) of grems standing at `foes_at`.
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
        let unit = grem(n as i64 + 1, n);
        columns.position[n] = Frame::kept(&squads, 0).place((n as i32, 0), unit.foot);
        roster.listed[0].living.push(unit);
    }
    for (k, &at) in foes_at.iter().enumerate() {
        let slot = columns.position.len();
        columns.grow();
        columns.position[slot] = at;
        columns.speed[slot] = 1.0;
        roster.listed[1].living.push(grem(101 + k as i64, slot));
    }
    squads.order[1] = locks::RETREAT;
    Built { squads, columns, roster }
}

/// Unit `n` of the player squad is out chasing the kingdom squad, `out` cells south of its
/// place, from where it stands, on `leash` cells.
fn send_from(b: &mut Built, n: usize, leash: f64, out: f32) {
    let unit = &b.roster.listed[0].living[n];
    let at = b.columns.position[unit.slot].add(V2::new(0.0, out));
    entries::loosen(&mut b.squads, &mut b.columns, 0, (unit.id, unit.slot), at);
    let chaser = Chaser { unit: unit.id, foe: 2, from: at, leash, returning: false };
    b.squads.chase(0, chaser);
}

/// As send_from, two cells out of its place.
fn send(b: &mut Built, n: usize, leash: f64) {
    send_from(b, n, leash, 2.0);
}

fn run(b: &mut Built) {
    let chasing = [Chasing { listed: 0, homes: vec![] }];
    chase(&mut b.squads, &mut b.columns, &b.roster, &chasing, None, &rules());
}

#[test]
fn a_chaser_walks_after_the_nearest_unit_of_its_quarry() {
    let mut b = built(1, &[V2::new(4.0, 0.5), V2::new(6.0, 0.5)]);
    send(&mut b, 0, 10.0);
    let was = b.columns.loose_at[0];
    run(&mut b);
    let now = b.columns.loose_at[0];
    assert!(now.distance_to(V2::new(4.0, 0.5)) < was.distance_to(V2::new(4.0, 0.5)));
    assert_eq!(b.columns.next[0], now);
    assert!(!b.squads.chasers[0][0].returning);
}

#[test]
fn a_chaser_at_its_leash_turns_back() {
    let mut b = built(1, &[V2::new(4.0, 0.5)]);
    send(&mut b, 0, 0.0);
    run(&mut b);
    assert!(b.squads.chasers[0][0].returning);
}

#[test]
fn a_chaser_that_loses_sight_of_its_quarry_turns_back() {
    let mut b = built(1, &[V2::new(12.0, 0.5)]);
    send(&mut b, 0, 50.0);
    run(&mut b);
    assert!(b.squads.chasers[0][0].returning);
}

#[test]
fn a_tired_chaser_gives_up() {
    let mut b = built(1, &[V2::new(4.0, 0.5)]);
    send(&mut b, 0, 50.0);
    b.columns.stamina[0] = 19.0;
    run(&mut b);
    assert!(b.squads.chasers[0][0].returning);
}

#[test]
fn a_chaser_of_a_squad_gone_from_the_lists_turns_back() {
    let mut b = built(1, &[V2::new(4.0, 0.5)]);
    send(&mut b, 0, 50.0);
    b.roster.listed.pop();
    run(&mut b);
    assert!(b.squads.chasers[0][0].returning);
}

#[test]
fn a_chaser_back_in_its_place_rejoins_its_formation() {
    let mut b = built(1, &[V2::new(4.0, 0.5)]);
    send_from(&mut b, 0, 0.0, 0.0);
    run(&mut b); // turns back where it stands, on its place
    assert!(b.squads.chasers[0].is_empty());
    assert!(b.squads.loose[0].is_empty());
}

#[test]
fn a_fallen_chaser_is_called_off() {
    let mut b = built(1, &[V2::new(4.0, 0.5)]);
    send(&mut b, 0, 50.0);
    b.roster.listed[0].living.clear();
    run(&mut b);
    assert!(b.squads.chasers[0].is_empty());
}

fn temptation(chances: &[(f64, f64)]) -> Temptation<'_> {
    Temptation { chances, within: 3.0, tick: 5, seed: 7 }
}

#[test]
fn units_near_a_retreat_break_ranks_by_their_chance_and_far_ones_never() {
    let mut b = built(2, &[V2::new(2.0, 0.5)]);
    b.columns.position[1] = V2::new(-9.0, 0.5); // the second stands far off
    let t = temptation(&[(1.0, 4.0), (1.0, 4.0)]);
    tempt(&mut b.squads, &mut b.columns, &b.roster, (0, 1), &t);
    assert_eq!(b.squads.loose[0], vec![1]);
    let chaser = b.squads.chasers[0][0];
    assert_eq!((chaser.unit, chaser.foe, chaser.leash), (1, 2, 4.0));
    assert_eq!(chaser.from, b.columns.position[0]);
    let mut none = built(1, &[V2::new(2.0, 0.5)]);
    let t = temptation(&[(0.0, 4.0)]);
    tempt(&mut none.squads, &mut none.columns, &none.roster, (0, 1), &t);
    assert!(none.squads.chasers[0].is_empty());
}

#[test]
fn a_formation_lags_while_a_unit_in_its_ranks_is_behind_its_place() {
    let mut b = built(2, &[]);
    assert!(!lagging(&b.squads, &b.columns, &b.roster, 0, 1.0));
    b.columns.position[1] = b.columns.position[1].sub(V2::new(2.0, 0.0));
    assert!(lagging(&b.squads, &b.columns, &b.roster, 0, 1.0));
    send(&mut b, 1, 5.0); // out chasing, it makes its own way
    assert!(!lagging(&b.squads, &b.columns, &b.roster, 0, 1.0));
}

#[test]
fn a_squads_spread_covers_its_units_where_they_stand() {
    let b = built(0, &[V2::new(4.0, 0.5), V2::new(6.0, 2.5)]);
    let spread = spread(&b.squads, &b.columns, &b.roster, 1).unwrap();
    assert_eq!(spread.position, V2::new(3.5, 0.0));
    assert_eq!(spread.end(), V2::new(6.5, 3.0));
    assert!(spread_of_none(&b));
}

fn spread_of_none(b: &Built) -> bool {
    spread(&b.squads, &b.columns, &b.roster, 0).is_none()
}

#[test]
fn chasers_pick_and_walk_the_same_whatever_the_lists_order() {
    let foes = [V2::new(4.0, 0.5), V2::new(4.0, 1.5), V2::new(6.0, 0.5)];
    let mut b = built(3, &foes);
    for n in 0..3 {
        send(&mut b, n, 10.0);
    }
    run(&mut b);
    let mut r = built(3, &foes);
    for n in 0..3 {
        send(&mut r, n, 10.0);
    }
    r.roster.listed.reverse();
    for listed in &mut r.roster.listed {
        listed.living.reverse();
    }
    let chasing = [Chasing { listed: 1, homes: vec![] }];
    chase(&mut r.squads, &mut r.columns, &r.roster, &chasing, None, &rules());
    assert_eq!(b.columns.loose_at, r.columns.loose_at);
}
