//! Contact's scenarios (Decisions 40, 47, 51, 78, 97 and 111), on squads built here: lines of
//! two grems abreast, their fronts placed by hand.

use super::*;
use crate::ground::Walker;
use crate::maths::V2;
use crate::roster::{Listed, Member};

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

pub(super) struct Built {
    pub(super) squads: Squads,
    pub(super) columns: Columns,
    pub(super) roster: Roster,
}

impl Built {
    fn field(&self) -> Field<'_> {
        Field { squads: &self.squads, columns: &self.columns, roster: &self.roster }
    }
}

/// A squad per `fronts` entry (front, heading, faction), ids from 1, each two grems abreast
/// in its front rank.
pub(super) fn built(fronts: &[(V2, f64, u32)]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (s, &(at, heading, faction)) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
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

pub(super) const EAST: f64 = 90.0;
pub(super) const WEST: f64 = 270.0;
pub(super) const NORTH: f64 = 0.0;

#[test]
fn two_fronts_face_to_face_within_reach_lock_onto_each_other() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(1.0, 0.0), WEST, 1)]);
    let picks = fronts(&mut b.squads, &mut b.columns, &b.roster, 7);
    assert_eq!(picks, vec![(0, 1), (1, 0)]);
    assert_eq!(b.squads.engaged_with, vec![2, 1]);
    assert_eq!(b.squads.state, vec![locks::FIGHTING, locks::FIGHTING]);
    assert_eq!(b.columns.attack_cooldown, vec![1, 1, 1, 1]);
}

#[test]
fn fronts_out_of_reach_do_not_lock() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(2.0, 0.0), WEST, 1)]);
    assert!(fronts(&mut b.squads, &mut b.columns, &b.roster, 7).is_empty());
}

#[test]
fn a_foe_getting_away_is_engaged_but_does_not_turn_to_fight() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(1.0, 0.0), WEST, 1)]);
    b.squads.order[1] = locks::RETREAT;
    let picks = fronts(&mut b.squads, &mut b.columns, &b.roster, 7);
    assert_eq!(picks, vec![(0, 1)]);
    assert_eq!(b.squads.engaged_with, vec![2, 0]);
}

#[test]
fn of_two_foes_as_near_the_lower_draw_is_picked_whatever_the_order() {
    let (one, middle, three) =
        ((V2::new(1.0, 0.0), WEST, 1), (V2::ZERO, EAST, 0), (V2::new(1.0, 0.5), WEST, 1));
    let lower = if contest::squad_draw(7, 1) < contest::squad_draw(7, 3) { 1 } else { 3 };
    for (listed, ids) in [([one, middle, three], [1, 2, 3]), ([three, middle, one], [3, 2, 1])] {
        let mut b = built(&listed);
        for (l, id) in b.roster.listed.iter_mut().zip(ids) {
            l.id = id; // each squad keeps its id however it is listed
        }
        let everyone: Vec<usize> = (0..3).collect();
        let foe = b.field().nearest_hostile(1, &everyone, 7).unwrap();
        assert_eq!(b.roster.listed[foe].id, lower);
    }
}

#[test]
fn an_advance_stops_at_reach_of_a_hostile_front_and_two_closing_split_the_gap() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(6.4, 0.0), WEST, 1)]);
    b.squads.direction[0] = 1;
    let field = b.field();
    let stop = field.limit(0, 1.0, &[]);
    assert!((stop - (0.1 - MELEE_REACH)).abs() < 0.000001, "{stop}");
    let halved = field.limit(0, 1.0, &[2]);
    assert!((halved - (0.1 - MELEE_REACH) / 2.0).abs() < 0.000001, "{halved}");
}

#[test]
fn an_advance_stops_behind_a_friends_back_rank() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(6.4, 0.0), EAST, 0)]);
    b.squads.direction[0] = 1;
    let stop = b.field().limit(0, 1.0, &[]);
    assert!((stop - (0.1 - RANK_DEPTH)).abs() < 0.000001, "{stop}");
}

#[test]
fn a_ranged_line_with_no_front_units_holds_to_skirmish_within_its_reach() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(5.0, 0.0), WEST, 1)]);
    for unit in &mut b.roster.listed[0].living {
        unit.preferred = 2;
        unit.attack_range = 6;
    }
    assert!(b.field().skirmishing(0));
    b.roster.listed[0].living[0].attack_range = 3;
    b.roster.listed[0].living[1].attack_range = 4;
    assert!(!b.field().skirmishing(0), "out of its reach");
    b.roster.listed[0].living[1].preferred = 0;
    b.roster.listed[0].living[1].attack_range = 6;
    assert!(!b.field().skirmishing(0), "a front unit marches on");
}

#[test]
fn a_wave_joins_a_fighting_friend_whose_back_rank_it_reached() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(1.0, 0.0), EAST, 0)]);
    assert_eq!(b.field().joinable(0), None, "a friend on the march: only one that merges joins");
    b.roster.listed[0].merges = true;
    assert_eq!(b.field().joinable(0), Some(1));
    b.roster.listed[0].merges = false;
    b.squads.state[1] = locks::FIGHTING;
    assert_eq!(b.field().joinable(0), Some(1));
}

#[test]
fn an_attacker_reaching_a_side_locks_on_there_and_its_edge_strikes() {
    // an east-facing line at (0, 0) spans y -1 to 1; an attacker heading north from y 2
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(-0.5, 2.0), NORTH, 1)]);
    let flanked = edges(&mut b.squads, &mut b.columns, &b.roster, 9, 7);
    assert_eq!(flanked, vec![Flanked { victim: 0, attacker: 1, edge: 1 }]);
    assert_eq!(b.squads.flanks[0][0].foe, 2);
    assert_eq!(b.squads.flanks[0][0].since, 9);
    assert_eq!(b.squads.state[0], locks::FIGHTING);
    assert_eq!(b.squads.engaged_with[1], 1);
    assert_eq!(b.columns.attack_cooldown[0..2], [0, 1]); // its right-hand unit, on that edge
}

#[test]
fn of_two_reaching_one_edge_the_nearer_holds_it() {
    let mut b = built(&[
        (V2::new(0.0, 0.0), EAST, 0),
        (V2::new(-0.5, 2.1), NORTH, 1),
        (V2::new(-0.5, 2.0), NORTH, 1),
    ]);
    let flanked = edges(&mut b.squads, &mut b.columns, &b.roster, 9, 7);
    assert_eq!(flanked.iter().map(|f| f.attacker).collect::<Vec<_>>(), vec![2, 1]);
    assert_eq!(b.squads.flanks[0].len(), 1);
    assert_eq!(b.squads.flanks[0][0].foe, 3);
}

// From test_formation_engage_rule.gd (Decision 111).

#[test]
fn any_standing_squad_may_be_engaged_but_only_one_not_getting_away_seeks_combat() {
    let mut b = built(&[(V2::ZERO, EAST, 0); 4]);
    let (holding, retreating, routing, destroyed) = (0, 1, 2, 3);
    b.squads.state[holding] = locks::HOLDING;
    b.squads.order[retreating] = locks::RETREAT;
    b.squads.state[routing] = locks::ROUTING;
    b.squads.state[destroyed] = locks::DESTROYED;
    let field = b.field();
    for squad in [holding, retreating, routing] {
        assert!(field.engageable(squad));
    }
    assert!(!field.engageable(destroyed));
    assert!(field.can_engage(holding));
    assert!(!field.can_engage(retreating));
    assert!(!field.can_engage(routing));
}

#[test]
fn a_routing_squad_is_met_where_its_units_are_not_at_its_frame() {
    let mut b = built(&[(V2::ZERO, EAST, 0), (V2::new(0.5, 0.0), WEST, 1)]);
    b.squads.state[1] = locks::ROUTING;
    let field = b.field();
    assert!(!field.framed(1));
    assert!(field.framed(0));
    assert_eq!(field.nearest_hostile(0, &[0, 1], 7), None);
}

#[test]
fn waves_reaching_one_friend_join_nearest_first_whatever_their_order() {
    let friend = (V2::new(1.0, 0.0), EAST, 0);
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0), (V2::new(0.5, 0.0), EAST, 0), friend]);
    b.squads.state[2] = locks::FIGHTING;
    assert_eq!(
        b.field().joins(&[0, 1], 7),
        vec![(2, 1), (2, 0)],
        "the nearer takes the rank behind"
    );
    assert_eq!(b.field().joins(&[1, 0], 7), vec![(2, 1), (2, 0)]);
}

#[test]
fn of_two_waves_as_near_the_lower_draw_joins_first_and_one_short_does_not_join() {
    let wave = (V2::new(0.0, 0.0), EAST, 0);
    let short = (V2::new(-4.0, 0.0), EAST, 0);
    let mut b = built(&[wave, wave, (V2::new(1.0, 0.0), EAST, 0), short]);
    b.squads.state[2] = locks::FIGHTING;
    let seed = 7;
    let first = if contest::squad_draw(seed, 1) < contest::squad_draw(seed, 2) { 0 } else { 1 };
    let joins = b.field().joins(&[3, 1, 0], seed);
    assert_eq!(joins, vec![(2, first), (2, 1 - first)]);
    assert_eq!(b.field().joins(&[0, 1, 3], seed), joins);
}
