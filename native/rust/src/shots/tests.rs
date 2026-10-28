//! FormationCombat's ranged scenarios (Decisions 46 and 47), on squads built here: the
//! player's (faction 0) at the origin facing east, the kingdom's (faction 1) `apart` cells
//! east of it facing west, their fronts towards each other. A unit's cooldown starts at 1
//! unless set, so a shooter that finds a target shoots at once.

use super::*;
use crate::ground::Walker;
use crate::maths::V2;
use crate::roster::{Listed, Member};

const SEED: i64 = 7;
const EAST: f64 = 90.0;
const WEST: f64 = 270.0;
const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

#[derive(Default)]
struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

impl Built {
    /// A squad of `faction` with its front at `at`, facing `heading`, `width` columns wide.
    fn squad(&mut self, faction: u32, at: V2, heading: f64, width: i32) -> usize {
        let n = self.roster.listed.len();
        self.squads.open(n as i64 + 1);
        self.squads.position[n] = at;
        self.squads.heading[n] = heading;
        self.squads.width[n] = width;
        self.roster.listed.push(Listed {
            squad: n,
            id: n as i64 + 1,
            faction,
            ..Listed::default()
        });
        n
    }

    /// A unit of squad `n` at `place` (rank, column) with a range of `range` ranks; its
    /// member index.
    fn unit(&mut self, n: usize, id: i64, place: (i32, i32), range: i64) -> usize {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.rank[slot] = place.0;
        self.columns.column[slot] = place.1;
        self.columns.attack_cooldown[slot] = 1;
        self.columns.target_id[slot] = -1;
        let member = Member {
            id,
            slot,
            foot: (1, 1),
            preferred: 0,
            attack_range: range,
            kind: 0,
            turn_rate: 360.0,
            backward_pace: 0.5,
            detection: 8.0,
            max_stamina: 100.0,
            walker: GREM,
        };
        self.roster.listed[n].living.push(member);
        self.roster.listed[n].living.len() - 1
    }

    /// The player's squad at the origin and the kingdom's `apart` cells east.
    fn facing(apart: f32) -> (Built, usize, usize) {
        let mut built = Built::default();
        let ours = built.squad(0, V2::ZERO, EAST, 1);
        let theirs = built.squad(1, V2::new(apart, 0.0), WEST, 1);
        (built, ours, theirs)
    }

    fn slot(&self, n: usize, u: usize) -> usize {
        self.roster.listed[n].living[u].slot
    }

    fn shoot(&mut self, shooters: &[Shooter]) -> Vec<(usize, usize, usize)> {
        shots(&self.squads, &mut self.columns, &self.roster, shooters, SEED)
    }
}

fn shooter(listed: usize, member: usize) -> Shooter {
    Shooter { listed, member, reset: 10 }
}

#[test]
fn a_shooter_behind_its_front_strikes_the_enemy_front_within_its_range() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    built.unit(ours, 1, (0, 0), 0);
    let spitter = built.unit(ours, 2, (2, 0), 5);
    let front = built.unit(theirs, 3, (0, 0), 0);
    built.unit(theirs, 4, (1, 0), 0);
    assert_eq!(built.shoot(&[shooter(ours, spitter)]), vec![(0, theirs, front)]);
    let slot = built.slot(ours, spitter);
    assert_eq!(built.columns.target_id[slot], 3);
    assert_eq!(built.columns.attack_cooldown[slot], 10);
}

#[test]
fn a_shooter_out_of_range_holds_its_fire() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    let spitter = built.unit(ours, 1, (2, 0), 4);
    built.unit(theirs, 2, (0, 0), 0);
    assert!(built.shoot(&[shooter(ours, spitter)]).is_empty());
    let slot = built.slot(ours, spitter);
    assert_eq!(built.columns.target_id[slot], -1);
    assert_eq!(built.columns.attack_cooldown[slot], 1);
}

#[test]
fn its_cooldown_runs_down_before_it_shoots_again() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    let spitter = built.unit(ours, 1, (0, 0), 5);
    built.unit(theirs, 2, (0, 0), 0);
    let slot = built.slot(ours, spitter);
    built.columns.attack_cooldown[slot] = 3;
    assert!(built.shoot(&[shooter(ours, spitter)]).is_empty());
    assert_eq!(built.columns.attack_cooldown[slot], 2);
    assert_eq!(built.columns.target_id[slot], 2);
    assert!(built.shoot(&[shooter(ours, spitter)]).is_empty());
    assert_eq!(built.shoot(&[shooter(ours, spitter)]).len(), 1);
    assert_eq!(built.columns.attack_cooldown[slot], 10);
}

#[test]
fn the_nearest_rank_is_struck() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    let spitter = built.unit(ours, 1, (0, 0), 8);
    built.unit(theirs, 2, (2, 0), 0);
    let nearest = built.unit(theirs, 3, (1, 0), 0);
    assert_eq!(built.shoot(&[shooter(ours, spitter)]), vec![(0, theirs, nearest)]);
}

#[test]
fn of_a_rank_one_it_overlaps_across_its_columns_is_struck_first() {
    let mut built = Built::default();
    let ours = built.squad(0, V2::ZERO, EAST, 1);
    let theirs = built.squad(1, V2::new(3.0, 0.0), WEST, 4);
    let spitter = built.unit(ours, 1, (0, 0), 5);
    let across: Vec<usize> = (0..4).map(|c| built.unit(theirs, 10 + c as i64, (0, c), 0)).collect();
    let shots = built.shoot(&[shooter(ours, spitter)]);
    let (_, n, u) = shots[0];
    let span = geometry::extent(
        &Shape::of(&built.squads, &built.columns, &built.roster.listed[ours])
            .corners(&built.roster.listed[ours].living[spitter]),
        lateral_axis(EAST),
    );
    let theirs_shape = Shape::of(&built.squads, &built.columns, &built.roster.listed[theirs]);
    let struck = geometry::extent(
        &theirs_shape.corners(&built.roster.listed[n].living[u]),
        lateral_axis(EAST),
    );
    assert!(across.contains(&u));
    assert!(geometry::overlap(span, struck) > EPSILON, "it struck one it overlaps");
}

#[test]
fn friends_and_squads_that_arrived_are_not_shot_at() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    let spitter = built.unit(ours, 1, (1, 0), 6);
    built.unit(ours, 2, (0, 0), 0);
    built.unit(theirs, 3, (0, 0), 0);
    built.squads.state[built.roster.listed[theirs].squad] = locks::ARRIVED;
    assert!(built.shoot(&[shooter(ours, spitter)]).is_empty());
}

#[test]
fn a_shot_is_the_same_whatever_the_lists_order() {
    let (mut forward, ours, theirs) = Built::facing(3.0);
    let spitter = forward.unit(ours, 1, (0, 0), 6);
    for c in 0..3 {
        forward.unit(theirs, 10 + c, (0, 0), 0);
        forward.unit(theirs, 20 + c, (1, 0), 0);
    }
    let mut reversed = Built::default();
    let theirs_r = reversed.squad(1, V2::new(3.0, 0.0), WEST, 1);
    let ours_r = reversed.squad(0, V2::ZERO, EAST, 1);
    for c in (0..3).rev() {
        reversed.unit(theirs_r, 20 + c, (1, 0), 0);
        reversed.unit(theirs_r, 10 + c, (0, 0), 0);
    }
    let spitter_r = reversed.unit(ours_r, 1, (0, 0), 6);
    let (_, n, u) = forward.shoot(&[shooter(ours, spitter)])[0];
    let (_, n_r, u_r) = reversed.shoot(&[shooter(ours_r, spitter_r)])[0];
    assert_eq!(forward.roster.listed[n].living[u].id, reversed.roster.listed[n_r].living[u_r].id);
}

// Who may shoot (FormationCombat.ranged_blows' choice, spec 39 step 8a2-v).

/// Every living unit's ranged time, one second each.
fn seconds(built: &Built) -> Vec<f64> {
    built.roster.listed.iter().flat_map(|l| l.living.iter().map(|_| 1.0)).collect()
}

fn chosen(built: &Built, seconds: &[f64], interval: i64) -> Vec<(usize, usize, i32)> {
    shooters(&built.squads, &built.columns, &built.roster, seconds, interval)
        .iter()
        .map(|s| (s.listed, s.member, s.reset))
        .collect()
}

#[test]
fn the_living_with_a_range_may_shoot_in_the_lists_order() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    built.unit(ours, 1, (0, 0), 0);
    let spitter = built.unit(ours, 2, (1, 0), 5);
    let theirs_spitter = built.unit(theirs, 3, (0, 0), 4);
    let all = seconds(&built);
    assert_eq!(chosen(&built, &all, 10), vec![(ours, spitter, 10), (theirs, theirs_spitter, 10)]);
}

#[test]
fn it_starts_again_at_its_ranged_time_in_ticks_rounded_and_at_least_one() {
    let (mut built, ours, _) = Built::facing(3.0);
    built.unit(ours, 1, (0, 0), 5);
    built.unit(ours, 2, (1, 0), 5);
    built.unit(ours, 3, (2, 0), 5);
    let resets: Vec<i32> =
        chosen(&built, &[1.25, 0.01, 0.35], 10).iter().map(|&(_, _, r)| r).collect();
    assert_eq!(resets, vec![13, 1, 4]);
}

#[test]
fn a_fighting_squads_front_rank_fights_as_melee_and_its_rear_shoots() {
    let (mut built, ours, _) = Built::facing(3.0);
    built.unit(ours, 1, (0, 0), 5);
    let rear = built.unit(ours, 2, (1, 0), 5);
    built.squads.state[built.roster.listed[ours].squad] = locks::FIGHTING;
    let all = seconds(&built);
    assert_eq!(chosen(&built, &all, 10), vec![(ours, rear, 10)]);
    built.squads.state[built.roster.listed[ours].squad] = locks::HOLDING;
    assert_eq!(chosen(&built, &all, 10).len(), 2);
}

#[test]
fn a_squad_that_arrived_holds_its_fire() {
    let (mut built, ours, theirs) = Built::facing(3.0);
    built.unit(ours, 1, (0, 0), 5);
    let spitter = built.unit(theirs, 2, (0, 0), 5);
    built.squads.state[built.roster.listed[ours].squad] = locks::ARRIVED;
    let all = seconds(&built);
    assert_eq!(chosen(&built, &all, 10), vec![(theirs, spitter, 10)]);
}
