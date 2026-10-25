//! A pursuit as a whole's scenarios (Decisions 95, 103, 107 and 113), on squads built here:
//! a player squad (id 1) of two grems pursuing east along a road on y = 0 from its post at
//! x = 10, and the kingdom squad (id 2) it pursues, retreating east, of two grems.

use super::*;
use crate::frame::Frame;
use crate::ground::Walker;
use crate::roster::{Listed, Member};
use crate::squads::Pursuit;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const HOLD: i32 = 1;

fn rules() -> Rules {
    Rules { cells_per_second: 20.0, seconds: 0.1, give_up: 0.2, lag: 2.0 }
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

fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(64.0, 0.0)], 0.0)
}

/// The pursuer (slot 0) at its post, its two grems at their places, and its quarry (slot 1)
/// of grems standing at `foes_at`, ordered to retreat.
fn built(foes_at: &[V2]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (slot, (id, faction)) in [(1i64, 0u32), (2, 1)].into_iter().enumerate() {
        squads.open(id);
        squads.width[slot] = 1;
        roster.listed.push(Listed { squad: slot, id, faction, ..Listed::default() });
    }
    squads.front_distance[0] = 10.0 / CELLS;
    squads.position[0] = V2::new(10.0, 0.0);
    squads.direction[0] = 1;
    squads.engaged_with[0] = 2;
    squads.pursuit[0] = Some(Pursuit {
        foe: 2,
        post: 10.0 / CELLS,
        post_at: V2::new(10.0, 0.0),
        home: 0.0,
        direction: 1,
        order: HOLD,
        returning: false,
    });
    for rank in 0..2 {
        columns.grow();
        columns.rank[rank] = rank as i32;
        let unit = grem(rank as i64 + 1, rank);
        columns.position[rank] = Frame::kept(&squads, 0).place((rank as i32, 0), unit.foot);
        roster.listed[0].living.push(unit);
    }
    for (k, &at) in foes_at.iter().enumerate() {
        let slot = columns.position.len();
        columns.grow();
        columns.position[slot] = at;
        roster.listed[1].living.push(grem(101 + k as i64, slot));
    }
    for slot in 0..columns.position.len() {
        columns.speed[slot] = 1.0;
        columns.stamina[slot] = 100.0;
    }
    squads.order[1] = locks::RETREAT;
    Built { squads, columns, roster }
}

fn ahead_of_it() -> Built {
    built(&[V2::new(14.0, 0.0), V2::new(15.0, 0.0)])
}

fn pursuing(leash: f64) -> Pursuing {
    Pursuing { listed: 0, route: Some(road()), held: Some(road()), leash }
}

fn run(b: &mut Built, p: &[Pursuing]) -> Vec<Pursued> {
    step(&mut b.squads, &b.columns, &b.roster, p, &rules())
}

#[test]
fn it_advances_along_its_route_after_its_enemy_at_its_pace() {
    let mut b = ahead_of_it();
    assert_eq!(run(&mut b, &[pursuing(f64::INFINITY)]), vec![]);
    assert_eq!(b.squads.front_distance[0], 12.0 / CELLS);
    assert_eq!(b.squads.position[0], V2::new(12.0, 0.0));
    assert_eq!(b.squads.heading[0], 90.0, "the road runs east");
}

#[test]
fn it_closes_no_nearer_than_a_cell_short_of_where_its_enemys_units_stand() {
    let mut b = built(&[V2::new(12.0, 0.0), V2::new(13.0, 0.0)]);
    run(&mut b, &[pursuing(f64::INFINITY)]);
    assert_eq!(b.squads.front_distance[0], 11.5 / CELLS, "1.5 short of 12.5, their middle");
    let mut b = built(&[V2::new(10.0, 0.0), V2::new(11.0, 0.0)]);
    run(&mut b, &[pursuing(f64::INFINITY)]);
    assert_eq!(b.squads.front_distance[0], 10.0 / CELLS, "within a cell: it holds");
}

#[test]
fn it_waits_for_its_rearmost_unit() {
    let mut b = ahead_of_it();
    b.columns.position[1] = b.columns.position[1].sub(V2::new(3.0, 0.0));
    run(&mut b, &[pursuing(f64::INFINITY)]);
    assert_eq!(b.squads.front_distance[0], 10.0 / CELLS);
}

/// Asserts it gave up: its fight over, it marches back to its post.
fn gave_up(b: &Built, out: &[Pursued]) {
    assert_eq!(out, &[Pursued::Ended(0)]);
    assert_eq!((b.squads.engaged_with[0], b.squads.order[0]), (0, locks::RETREAT));
    assert_eq!(b.squads.home_distance[0], 10.0 / CELLS, "home: its post");
    assert!(b.squads.pursuit[0].is_some_and(|p| p.returning));
    assert_eq!(b.squads.front_distance[0], 10.0 / CELLS, "it went nowhere this tick");
}

#[test]
fn it_gives_up_on_an_enemy_no_longer_retreating_gone_or_destroyed() {
    let mut b = ahead_of_it();
    b.squads.order[1] = HOLD;
    let out = run(&mut b, &[pursuing(f64::INFINITY)]);
    gave_up(&b, &out);
    let mut b = built(&[]);
    let out = run(&mut b, &[pursuing(f64::INFINITY)]);
    gave_up(&b, &out);
    let mut b = ahead_of_it();
    b.roster.listed.pop();
    let out = run(&mut b, &[pursuing(f64::INFINITY)]);
    gave_up(&b, &out);
}

#[test]
fn it_gives_up_on_an_enemy_out_of_its_sight() {
    let mut b = built(&[V2::new(30.0, 0.0)]);
    let out = run(&mut b, &[pursuing(f64::INFINITY)]);
    gave_up(&b, &out);
}

#[test]
fn it_gives_up_once_most_of_it_has_tired() {
    let mut b = ahead_of_it();
    b.columns.stamina[0] = 10.0;
    assert_eq!(run(&mut b, &[pursuing(f64::INFINITY)]), vec![], "one of two: not most");
    let mut b = ahead_of_it();
    b.columns.stamina[0] = 10.0;
    b.columns.stamina[1] = 10.0;
    let out = run(&mut b, &[pursuing(f64::INFINITY)]);
    gave_up(&b, &out);
}

#[test]
fn it_gives_up_at_its_leash_from_its_post() {
    let mut b = ahead_of_it();
    assert_eq!(run(&mut b, &[pursuing(1.5)]), vec![]);
    let out = run(&mut b, &[pursuing(1.5)]);
    assert_eq!(out, vec![Pursued::Ended(0)], "2 cells out after one tick");
    let mut b = ahead_of_it();
    let out = run(&mut b, &[pursuing(0.0)]);
    gave_up(&b, &out);
}

#[test]
fn going_back_it_waits_for_its_march_home_then_takes_up_its_post_and_order_again() {
    let mut b = ahead_of_it();
    let held = b.squads.pursuit[0].unwrap();
    b.squads.pursuit[0] = Some(Pursuit { home: 0.25, direction: -1, returning: true, ..held });
    b.squads.order[0] = locks::RETREAT;
    b.squads.front_distance[0] = 0.5;
    let own = Route::new(vec![V2::new(0.0, 4.0), V2::new(64.0, 4.0)], 0.0);
    let p = Pursuing { listed: 0, route: Some(road()), held: Some(own), leash: f64::INFINITY };
    assert_eq!(run(&mut b, std::slice::from_ref(&p)), vec![], "still marching back");
    b.squads.order[0] = 0; // ADVANCE: its march home ended
    assert_eq!(run(&mut b, &[p]), vec![Pursued::Arrived(0, -1)]);
    assert!(b.squads.pursuit[0].is_none());
    assert_eq!((b.squads.front_distance[0], b.squads.home_distance[0]), (10.0 / CELLS, 0.25));
    assert_eq!(b.squads.position[0], V2::new(10.0, 4.0), "on its own route");
    assert_eq!(b.squads.order[0], HOLD);
}

#[test]
fn each_pursuer_decides_alone_whoever_is_listed_first() {
    for reversed in [false, true] {
        let mut b = ahead_of_it();
        let mut p = pursuing(f64::INFINITY);
        if reversed {
            b.roster.listed.reverse();
            p.listed = 1;
        }
        run(&mut b, &[p]);
        assert_eq!(b.squads.front_distance[0], 12.0 / CELLS, "reversed {reversed}");
    }
}
