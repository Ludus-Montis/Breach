//! The walk's share over a squad built here: three grems abreast facing east, each standing
//! at its place unless a test moves it, on open ground.

use super::*;
use crate::ground::Walker;
use crate::roster::Member;
use crate::swaps::{Held, HeldUnit};

const EAST: f64 = 90.0;
/// [discipline_per_leadership, walk_slack_loose, walk_slack_drilled]: a slack of 1 cell at
/// discipline 0.
const SLACK: [f64; 3] = [0.0, 1.0, 4.0];

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
    field: Field,
    listed: Listed,
}

/// Squad 1 of three grems (ids 10, 11, 12) abreast, each at its place.
fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(1);
    squads.heading[0] = EAST;
    squads.width[0] = 3;
    squads.position[0] = V2::new(40.0, 8.0);
    let mut living = Vec::new();
    for column in 0..3 {
        let slot = field.hold(10 + column) as usize;
        field.columns.column[slot] = column as i32;
        field.columns.sheet[slot].discipline = 0; // a slack of 1 cell
        squads.members[0].push(slot as i32);
        living.push(grem(10 + column, slot));
    }
    let listed = Listed { squad: 0, id: 1, living, ..Listed::default() };
    let mut b = Built { squads, field, listed };
    b.stand_at_places();
    b
}

impl Built {
    fn stand_at_places(&mut self) {
        let frame = self.frame();
        let framed: Vec<(i32, i64)> = (0..3).map(|n| (n, 10 + i64::from(n))).collect();
        for (n, unit) in pourers(&self.squads, &self.field, 0, &framed, false).iter().enumerate() {
            self.field.columns.position[n] = pour::place(&frame, unit);
        }
    }

    fn frame(&self) -> Frame {
        Frame { anchor: self.squads.position[0], heading: EAST, width: 3, centre_shift: 0.0 }
    }

    /// The share with walk_lost `lost`.
    fn share(&self, lost: f64) -> f64 {
        let t = ShareTuning { seed: 7, wounds_reach: 1.0, lost, slack: SLACK };
        of(&self.squads, &self.field, &self.listed, None, &t)
    }

    /// Unit `n` stands `by` cells behind where it stands (west, the frame facing east).
    fn lag(&mut self, n: usize, by: f32) {
        self.field.columns.position[n].x -= by;
    }

    fn give_trait(&mut self, n: usize, name: &str) {
        let code = self.field.lexicon.code(name);
        self.field.columns.words[n].traits.push((code, 1.0));
    }
}

#[test]
fn every_unit_at_its_place_keeps_the_whole_step() {
    assert_eq!(built().share(2.0), 1.0);
}

#[test]
fn a_unit_lagging_three_quarters_of_its_slack_halves_the_step_and_one_at_its_slack_stops_it() {
    let mut b = built();
    b.lag(1, 0.75);
    assert!((b.share(2.0) - 0.5).abs() < 1e-6);
    b.lag(1, 0.25);
    assert_eq!(b.share(2.0), 0.0);
}

#[test]
fn one_fallen_behind_is_waited_for_only_if_no_man_is_left_behind() {
    let mut b = built();
    b.lag(2, 1.5);
    assert_eq!(b.share(1.25), 1.0, "past walk_lost, it is left");
    for n in 0..3 {
        b.give_trait(n, NO_MAN_LEFT);
    }
    assert_eq!(b.share(1.25), 0.0, "every unit has no man left behind");
}

#[test]
fn a_leader_who_leaves_the_fallen_behind_never_waits() {
    let mut b = built();
    b.lag(0, 0.9);
    b.give_trait(0, LEFT_BEHIND);
    assert!(b.share(2.0) < 1.0, "a follower's trait isn't the squad's");
    b.field.columns.sheet[0].leadership = 1;
    assert_eq!(b.share(2.0), 1.0);
}

#[test]
fn a_unit_part_way_through_a_swap_is_judged_from_its_place_part_way_there() {
    let mut b = built();
    let unit = HeldUnit { id: 11, slot: 1, from: (0, 1), to: (1, 1), foot: (1, 1) };
    b.squads.swaps[0] = vec![Held { ticks: 1, total: 2, units: vec![unit], passed: Vec::new() }];
    b.lag(1, 0.75); // a quarter behind where its swap, half a rank back, has brought it
    assert_eq!(b.share(2.0), 1.0, "three quarters behind its place in the frame would halve it");
}

#[test]
fn units_loose_or_fleeing_are_not_waited_for_and_one_out_taking_holds_the_frame() {
    let mut b = built();
    b.lag(0, 2.0);
    b.lag(2, 2.0);
    b.squads.loose[0] = vec![10];
    b.squads.fleeing[0] = vec![12];
    assert_eq!(b.share(4.0), 1.0);
    b.squads.taking[0] = vec![(11, 0)];
    assert_eq!(b.share(4.0), 0.0);
}

#[test]
fn a_drilled_squad_has_more_slack() {
    let mut b = built();
    b.lag(1, 1.5);
    assert_eq!(b.share(4.0), 0.0);
    for n in 0..3 {
        b.field.columns.sheet[n].discipline = 100;
    }
    assert!((b.share(4.0) - 1.0).abs() < 1e-6, "a slack of 4: within half of it");
}
