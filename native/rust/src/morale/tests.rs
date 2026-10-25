//! FormationMorale's pressure and recovery (Decision 82), from test_formation_morale.gd, on
//! squads built here: one-cell units, each squad facing east, its front's centre at `at`.
//! Faction 0 is the player's, 1 the kingdom's. The tuning is the repo's: pressure [0, 0, 3,
//! 9, 18], recovery 2, support 2 cells, condition at most 1.25.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::Walker;
use crate::maths::V2;
use crate::roster::Member;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn tuning() -> MoraleTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    MoraleTuning::of(&content.tuning)
}

#[derive(Default)]
struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

impl Built {
    /// A squad of `faction` facing east, its front's centre at `at`, one column wide, of one
    /// unit at (0, 0) whose condition is `condition`; its index in the roster.
    fn squad(&mut self, faction: u32, at: (f32, f32), condition: f64) -> usize {
        let n = self.roster.listed.len();
        let id = n as i64 + 1;
        let squad = self.squads.open(id) as usize;
        self.squads.position[squad] = V2::new(at.0, at.1);
        self.squads.heading[squad] = 90.0;
        self.squads.width[squad] = 1;
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.condition[slot] = condition;
        let unit = Member {
            id: 100 + id,
            slot,
            foot: (1, 1),
            preferred: 0,
            attack_range: 0,
            kind: 0,
            turn_rate: 360.0,
            backward_pace: 0.5,
            detection: 8.0,
            max_stamina: 100.0,
            walker: GREM,
        };
        self.roster.listed.push(Listed {
            squad,
            id,
            faction,
            living: vec![unit],
            ..Listed::default()
        });
        n
    }

    fn engaged(&mut self, n: usize) {
        self.squads.engaged_with[self.roster.listed[n].squad] = 99;
    }

    fn flanked(&mut self, n: usize, edge: i32) {
        self.squads.flank(self.roster.listed[n].squad, edge, 98, 0);
    }

    fn run(&self, stepping: &[Stepping]) -> Vec<i64> {
        step(&self.squads, &self.columns, &self.roster, stepping, &tuning())
    }
}

fn at(listed: usize, morale: i64) -> Stepping {
    Stepping { listed, morale, leadership: 0, ceiling: 70 }
}

#[test]
fn pressure_grows_with_the_sides_fought_on() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    assert_eq!(b.run(&[at(squad, 60)]), [60]);
    b.flanked(squad, RIGHT);
    assert_eq!(b.run(&[at(squad, 60)]), [57]);
    b.flanked(squad, 2);
    assert_eq!(b.run(&[at(squad, 60)]), [51]);
}

#[test]
fn a_flank_alone_is_fought_on_as_one_side() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.flanked(squad, 2);
    assert_eq!(b.run(&[at(squad, 60)]), [60]);
}

#[test]
fn a_friend_beside_a_fought_side_takes_its_pressure() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    b.flanked(squad, RIGHT);
    b.squad(1, (10.0, 11.5), 1.0); // south: beside its right face
    assert_eq!(b.run(&[at(squad, 60)]), [60]);
}

#[test]
fn a_friend_on_the_other_side_a_foe_or_one_too_far_does_not() {
    for (faction, friend_at) in [(1, (10.0, 8.5)), (0, (10.0, 11.5)), (1, (10.0, 14.0))] {
        let mut b = Built::default();
        let squad = b.squad(1, (10.0, 10.0), 1.0);
        b.engaged(squad);
        b.flanked(squad, RIGHT);
        b.squad(faction, friend_at, 1.0);
        assert_eq!(b.run(&[at(squad, 60)]), [57], "{faction} at {friend_at:?}");
    }
}

#[test]
fn a_destroyed_friend_does_not_cover_a_side() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    b.flanked(squad, RIGHT);
    let friend = b.squad(1, (10.0, 11.5), 1.0);
    b.roster.listed[friend].living.clear();
    assert_eq!(b.run(&[at(squad, 60)]), [57]);
}

#[test]
fn a_friend_on_each_fought_flank_covers_both() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    b.flanked(squad, RIGHT);
    b.flanked(squad, LEFT);
    b.squad(1, (10.0, 11.5), 1.0);
    assert_eq!(b.run(&[at(squad, 60)]), [57]);
    b.squad(1, (10.0, 8.5), 1.0);
    assert_eq!(b.run(&[at(squad, 60)]), [60]);
}

#[test]
fn out_of_contact_morale_recovers_by_its_leadership_to_its_ceiling() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    let led = Stepping { leadership: 1, ..at(squad, 40) };
    assert_eq!(b.run(&[led]), [43]);
    assert_eq!(b.run(&[Stepping { morale: 69, ..led }]), [70]);
}

#[test]
fn a_poor_condition_recovers_less_and_a_hardened_one_at_most_its_cap() {
    let mut b = Built::default();
    let poor = b.squad(1, (10.0, 10.0), 0.2);
    let hard = b.squad(1, (30.0, 10.0), 3.0);
    assert_eq!(b.run(&[at(poor, 40), at(hard, 40)]), [40, 43]);
}

#[test]
fn the_same_squads_listed_in_reverse_step_alike() {
    let mut b = Built::default();
    let squad = b.squad(1, (10.0, 10.0), 1.0);
    b.engaged(squad);
    b.flanked(squad, RIGHT);
    let friend = b.squad(1, (10.0, 11.5), 0.5);
    let forward = b.run(&[at(squad, 60), at(friend, 30)]);
    b.roster.listed.reverse();
    let reversed = b.run(&[at(1, 60), at(0, 30)]);
    assert_eq!(forward, [60, 31]);
    assert_eq!(reversed, forward);
}
