//! The move phase's choices (spec 39 step 8b1), on squads built here: lines of two grems
//! abreast, their fronts placed by hand, as contact's tests build them.

use super::*;
use crate::contact::MELEE_REACH;
use crate::ground::Walker;
use crate::locks::RETREAT;
use crate::maths::V2;
use crate::roster::{Listed, Member};
use crate::squads::Chaser;

const EAST: f64 = 90.0;
const WEST: f64 = 270.0;

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

impl Built {
    fn field(&self) -> Field<'_> {
        Field { squads: &self.squads, columns: &self.columns, roster: &self.roster }
    }
}

/// A squad per `fronts` entry (front, heading, faction), ids from 1, each two grems abreast
/// in its front rank, marching the way it faces.
fn built(fronts: &[(V2, f64, u32)]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (s, &(at, heading, faction)) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
        squads.direction[s] = if heading == EAST { 1 } else { -1 };
        squads.width[s] = 2;
        let mut living = Vec::new();
        for column in 0..2 {
            let slot = columns.position.len();
            columns.grow();
            columns.column[slot] = column;
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

fn one() -> Built {
    built(&[(V2::new(0.0, 0.0), EAST, 0)])
}

#[test]
fn a_fighting_routing_arrived_or_destroyed_squad_stands_and_the_rest_may_go() {
    let mut b = one();
    for state in [FIGHTING, ROUTING, ARRIVED, DESTROYED] {
        b.squads.state[0] = state;
        assert_eq!(choose(&b.squads, 0), Choice::Stands, "state {state}");
    }
    for state in [MOVING, HOLDING] {
        b.squads.state[0] = state;
        assert_eq!(choose(&b.squads, 0), Choice::Goes, "state {state}");
    }
}

#[test]
fn a_squad_with_a_unit_out_of_its_place_regroups_but_not_for_a_chaser() {
    let mut b = one();
    b.squads.loose[0] = vec![100];
    assert!(regrouping(&b.squads, 0));
    assert_eq!(choose(&b.squads, 0), Choice::Stands);
    let chaser =
        Chaser { unit: 100, foe: 2, from: V2::new(0.0, 0.0), leash: 4.0, returning: false };
    b.squads.chase(0, chaser);
    assert!(!regrouping(&b.squads, 0), "out chasing on its own: it doesn't hold the squad");
    assert_eq!(choose(&b.squads, 0), Choice::Goes);
    b.squads.loose[0].push(101);
    assert!(regrouping(&b.squads, 0), "one loose unit not chasing is enough");
}

#[test]
fn a_squad_still_waiting_waits_unless_it_stands() {
    let mut b = one();
    b.squads.wait_ticks[0] = 3;
    assert_eq!(choose(&b.squads, 0), Choice::Waits);
    b.squads.loose[0] = vec![100];
    assert_eq!(choose(&b.squads, 0), Choice::Stands, "regrouping comes first");
    b.squads.loose[0].clear();
    b.squads.state[0] = FIGHTING;
    assert_eq!(choose(&b.squads, 0), Choice::Stands, "standing comes first");
}

#[test]
fn a_squad_ordered_to_hold_holds_and_one_free_to_march_moves() {
    let mut b = one();
    b.squads.order[0] = HOLD;
    assert!(hold(&mut b.squads, &b.columns, &b.roster, 0));
    assert_eq!(b.squads.state[0], HOLDING);
    b.squads.order[0] = ADVANCE;
    assert!(!hold(&mut b.squads, &b.columns, &b.roster, 0));
    assert_eq!(b.squads.state[0], MOVING);
}

#[test]
fn a_ranged_line_with_an_enemy_within_its_reach_holds_to_skirmish() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(5.0, 0.0), WEST, 1)]);
    for unit in &mut b.roster.listed[0].living {
        unit.preferred = 2;
        unit.attack_range = 6;
    }
    assert!(hold(&mut b.squads, &b.columns, &b.roster, 0));
    assert_eq!(b.squads.state[0], HOLDING);
}

#[test]
fn two_hostiles_advancing_at_each_other_each_close_half_the_gap_as_they_stood() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(6.4, 0.0), WEST, 1)]);
    b.squads.front_distance[1] = 1.0;
    let half = (0.1 - MELEE_REACH) / 2.0;
    let plans = [Plan { listed: 0, next: 0.5 }, Plan { listed: 1, next: 0.5 }];
    let landed = land(&b.field(), &plans);
    assert!((landed[0].0 - half).abs() < 0.000001, "{landed:?}");
    assert!((landed[1].0 - (1.0 - half)).abs() < 0.000001, "{landed:?}");
    assert!(landed[0].1 && landed[1].1, "both advance");
    let reversed = land(&b.field(), &[plans[1], plans[0]]);
    assert_eq!(reversed, vec![landed[1], landed[0]], "however they are listed");
}

#[test]
fn a_squad_not_advancing_lands_where_it_wanted_and_isnt_counted_marching() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(6.4, 0.0), WEST, 1)]);
    b.squads.front_distance[1] = 1.0;
    b.squads.order[1] = RETREAT;
    let plans = [Plan { listed: 0, next: 0.5 }, Plan { listed: 1, next: 1.2 }];
    let landed = land(&b.field(), &plans);
    assert_eq!(landed[1], (1.2, false), "a retreat goes where it wanted");
    // the retreating squad can't be engaged: the advance doesn't stop for it
    assert_eq!(landed[0], (0.5, true));
}

#[test]
fn an_advance_alone_stops_at_reach_of_a_standing_hostile() {
    let b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(6.4, 0.0), WEST, 1)]);
    let landed = land(&b.field(), &[Plan { listed: 0, next: 1.0 }]);
    assert!((landed[0].0 - (0.1 - MELEE_REACH)).abs() < 0.000001, "{landed:?}");
}
