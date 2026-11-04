//! The scrum pass's scenarios: squads of grems built here, their units placed by hand.

use super::*;
use crate::ground::Walker;
use crate::locks::FIGHTING;
use crate::manoeuvre::{COMBAT, ORDER, RE_FORM, WITHDRAW};
use crate::maths::V2;
use crate::roster::{Listed, Member};

const EAST: f64 = 90.0;
const WEST: f64 = 270.0;
const REACH: f64 = 0.5;

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
        walker: Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false },
    }
}

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

/// A squad per `fronts` entry (front, heading, faction), ids from 1, each two grems abreast
/// standing on its front at columns 0 and 1 across it.
fn built(fronts: &[(V2, f64, u32)]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (s, &(at, heading, faction)) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
        squads.width[s] = 2;
        squads.faction[s] = faction;
        let mut living = Vec::new();
        for column in 0..2 {
            let slot = columns.position.len();
            columns.grow();
            columns.column[slot] = column;
            columns.position[slot] = V2::new(at.x, at.y + column as f32);
            living.push(grem(100 * (s as i64 + 1) + column as i64, slot));
        }
        roster.listed.push(Listed {
            squad: s,
            id: s as i64 + 1,
            faction,
            living,
            ..Listed::default()
        });
    }
    Built { squads, columns, roster }
}

fn engaged(tick: i64, id: i64, faction: i64, foe: i64) -> [i64; 5] {
    [events::ENGAGED as i64, tick, id, faction, foe]
}

#[test]
fn squads_whose_units_come_within_reach_lock_and_each_lock_is_its_engaged_event() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(1.4, 0.0), WEST, 1)]);
    let mut log = Log::default();
    engage(&mut b.squads, &mut b.columns, &b.roster, (3, 7, REACH), &mut log);
    assert_eq!(log.take(), [engaged(3, 1, 0, 2), engaged(3, 2, 1, 1)].concat());
    assert_eq!(b.squads.engaged_with, vec![2, 1]);
    assert_eq!(b.squads.state, vec![FIGHTING, FIGHTING]);
}

#[test]
fn a_loose_unit_engages_from_where_it_stands_loose() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(9.0, 0.0), WEST, 1)]);
    let mut log = Log::default();
    engage(&mut b.squads, &mut b.columns, &b.roster, (3, 7, REACH), &mut log);
    assert!(log.take().is_empty());
    b.squads.loose[0].push(100);
    b.columns.loose_at[0] = V2::new(7.6, 0.0);
    engage(&mut b.squads, &mut b.columns, &b.roster, (4, 7, REACH), &mut log);
    assert_eq!(log.take(), [engaged(4, 1, 0, 2), engaged(4, 2, 1, 1)].concat());
}

#[test]
fn each_squad_takes_the_manoeuvre_with_the_highest_priority_that_applies() {
    let mut b = built(&[
        (V2::new(0.0, 0.0), EAST, 0),
        (V2::new(20.0, 0.0), WEST, 1),
        (V2::new(40.0, 0.0), EAST, 0),
        (V2::new(60.0, 0.0), EAST, 0),
    ]);
    b.squads.withdraw[0] = Some(0);
    b.squads.engaged_with[1] = 9;
    b.squads.narrow_ticks[2] = 4;
    manoeuvre(&mut b.squads, &mut b.columns, &b.roster);
    assert_eq!(b.squads.manoeuvre, vec![WITHDRAW, COMBAT, RE_FORM, ORDER]);
    assert_eq!(b.squads.narrow_ticks[2], 0);
    assert_eq!(b.squads.loose[2], vec![300, 301]);
}

#[test]
fn an_advancing_squad_of_archers_with_a_foe_in_its_reach_skirmishes() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(3.0, 0.0), WEST, 1)]);
    for m in &mut b.roster.listed[0].living {
        m.preferred = 1;
        m.attack_range = 4;
    }
    manoeuvre(&mut b.squads, &mut b.columns, &b.roster);
    assert_eq!(b.squads.manoeuvre, vec![COMBAT, ORDER]);
}

#[test]
fn a_fight_quiet_for_its_ticks_is_released_and_each_side_disengaged() {
    let mut b = built(&[
        (V2::new(0.0, 0.0), EAST, 0),
        (V2::new(1.4, 0.0), WEST, 1),
        (V2::new(20.0, 0.0), EAST, 0),
        (V2::new(21.4, 0.0), WEST, 1),
    ]);
    let mut log = Log::default();
    engage(&mut b.squads, &mut b.columns, &b.roster, (3, 7, REACH), &mut log);
    log.take();
    stall(&mut b.squads, &b.roster, (&[3], 2), 4, &mut log);
    assert!(log.take().is_empty());
    stall(&mut b.squads, &b.roster, (&[3], 2), 5, &mut log);
    let disengaged = events::DISENGAGED as i64;
    assert_eq!(log.take(), vec![disengaged, 5, 1, 0, disengaged, 5, 2, 1]);
    assert_eq!(b.squads.engaged_with, vec![0, 0, 4, 3]);
}
