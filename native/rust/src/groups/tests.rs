//! FormationGroups' scenarios (spec 30 round 3, part 4) and GroupPairs' (spec 38 step 1a), on
//! lines of grems built here: each line facing east, its units in rank 0 a cell apart
//! across, its front `front` cells along x, all of one faction, holding after a fight.

use super::*;
use crate::ground::Walker;
use crate::roster::{Listed, Member};

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const JOIN: f64 = 2.0;
const SEED: i64 = 7;

#[derive(Clone)]
struct Line {
    id: i64,
    count: usize,
    front: f32,
    command: u32,
    leads: bool,
    joins: bool,
    gathers: bool,
    merges: bool,
    fought: bool,
}

fn line(id: i64, count: usize, front: f32) -> Line {
    Line {
        id,
        count,
        front,
        command: id as u32,
        leads: false,
        joins: false,
        gathers: false,
        merges: false,
        fought: true,
    }
}

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
    groups: Vec<Group>,
}

/// The lines, in their order; unit k of a line has id 100 × its line's id + k. Its first
/// unit leads (and joins or gathers) if the line says so.
fn built(lines: &[Line]) -> Built {
    let mut b = Built {
        squads: Squads::default(),
        columns: Columns::default(),
        roster: Roster::default(),
        groups: vec![],
    };
    for (slot, l) in lines.iter().enumerate() {
        b.squads.open(l.id);
        b.squads.position[slot] = V2::new(l.front, 0.0);
        b.squads.width[slot] = l.count as i32;
        b.squads.state[slot] = HOLDING;
        b.squads.order[slot] = HOLD;
        b.squads.direction[slot] = 1;
        b.squads.fought[slot] = l.fought as i32;
        let mut living = vec![];
        let mut followers = vec![];
        for k in 0..l.count {
            let unit = b.columns.position.len();
            b.columns.grow();
            b.columns.position[unit] = V2::new(l.front - 0.5, k as f32);
            living.push(Member {
                id: 100 * l.id + k as i64,
                slot: unit,
                foot: (1, 1),
                preferred: 0,
                attack_range: 0,
                kind: 0,
                turn_rate: 360.0,
                backward_pace: 0.5,
                detection: 8.0,
                max_stamina: 100.0,
                walker: GREM,
            });
            let first = k == 0;
            followers.push(Follower {
                leads: first && l.leads,
                joins: first && l.joins,
                gathers: first && l.gathers,
                origin: Some(l.command),
            });
        }
        b.roster.listed.push(Listed {
            squad: slot,
            id: l.id,
            faction: 0,
            merges: l.merges,
            living,
            ..Listed::default()
        });
        b.groups.push(Group { command: l.command, route: 0, followers });
    }
    b
}

/// The meetings, with the groups named by their squads' ids.
fn meet(b: &Built) -> Vec<Meeting> {
    let id = |g: usize| b.roster.listed[g].id as usize;
    form_up(&b.squads, &b.columns, &b.roster, &b.groups, JOIN, SEED)
        .into_iter()
        .map(|m| match m {
            Meeting::Taken { taker, taken } => {
                Meeting::Taken { taker: id(taker), taken: id(taken) }
            }
            Meeting::Returned { from, home, units } => {
                Meeting::Returned { from: id(from), home: id(home), units }
            }
        })
        .collect()
}

fn taken(taker: usize, taken: usize) -> Meeting {
    Meeting::Taken { taker, taken }
}

#[test]
fn one_commands_groups_meeting_are_one_group_again_the_larger_taking_in() {
    let mut part = line(2, 2, 38.5);
    part.command = 1; // split off before
    assert_eq!(meet(&built(&[line(1, 4, 40.0), part])), vec![taken(1, 2)]);
}

#[test]
fn a_leader_and_a_leaderless_group_keep_their_own_orders() {
    let led = Line { leads: true, ..line(1, 2, 40.0) };
    assert!(meet(&built(&[led, line(2, 4, 38.5)])).is_empty());
}

#[test]
fn a_leader_that_gathers_takes_in_a_leaderless_group() {
    let led = Line { leads: true, gathers: true, ..line(1, 2, 40.0) };
    assert_eq!(meet(&built(&[led, line(2, 4, 38.5)])), vec![taken(1, 2)]);
}

#[test]
fn a_leader_that_joins_brings_its_group_into_the_others_command() {
    let joiner = Line { leads: true, joins: true, ..line(1, 3, 40.0) };
    assert_eq!(meet(&built(&[joiner, line(2, 2, 38.5)])), vec![taken(2, 1)]);
}

#[test]
fn forces_posted_near_each_other_that_have_not_fought_stay_apart() {
    let led = Line { leads: true, fought: false, ..line(1, 2, 40.0) };
    let other = Line { fought: false, merges: true, ..line(2, 3, 38.5) };
    assert!(meet(&built(&[led, other])).is_empty());
}

#[test]
fn two_leaderless_groups_stay_apart_unless_one_merges_the_larger_taking_in() {
    assert!(meet(&built(&[line(1, 2, 40.0), line(2, 3, 38.5)])).is_empty());
    let small = Line { merges: true, ..line(1, 2, 40.0) };
    assert_eq!(meet(&built(&[small, line(2, 3, 38.5)])), vec![taken(2, 1)]);
}

#[test]
fn groups_too_far_apart_do_not_meet() {
    let small = Line { merges: true, ..line(1, 2, 40.0) };
    assert!(meet(&built(&[small, line(2, 3, 30.0)])).is_empty());
}

#[test]
fn two_led_groups_keep_their_own_and_units_go_back_where_they_started() {
    let mut b = built(&[
        Line { leads: true, ..line(1, 3, 40.0) },
        Line { leads: true, ..line(2, 3, 38.5) },
    ]);
    b.groups[0].followers[2].origin = Some(2); // it started under the second's command
    assert_eq!(meet(&b), vec![Meeting::Returned { from: 1, home: 2, units: vec![102] }]);
}

#[test]
fn a_group_taken_in_is_met_no_more_and_the_taker_meets_with_what_it_took() {
    // the nearest meet first: 2 takes in 3 (one command), then 1 (a leader that gathers)
    // meets 2, now led by 3's leader that came with it, and leaves it be
    let first = Line { leads: true, gathers: true, ..line(1, 2, 43.0) };
    let mut third = Line { leads: true, ..line(3, 1, 38.5) };
    third.command = 2;
    let got = meet(&built(&[first, line(2, 4, 40.0), third]));
    assert_eq!(got, vec![taken(2, 3)]);
}

#[test]
fn the_same_meetings_decide_the_same_whatever_the_list_order() {
    let lines: Vec<Line> = [(1, 40.0), (2, 38.5), (3, 37.0)]
        .iter()
        .map(|&(id, front)| Line { merges: true, ..line(id, 2, front) })
        .collect();
    let mut reversed = lines.clone();
    reversed.reverse();
    let forward = meet(&built(&lines));
    assert!(!forward.is_empty());
    assert_eq!(forward, meet(&built(&reversed)));
}

#[test]
fn the_pairs_in_reach_come_in_list_order() {
    let points = [V2::new(0.0, 0.0), V2::new(30.0, 0.0), V2::new(9.0, 0.0), V2::new(-9.0, 1.0)];
    assert_eq!(pairs_near(&points, &[4.0; 4], 2.0), vec![(0, 2), (0, 3)]);
}

/// A small seeded generator for scattered tests: the same numbers on every run.
fn scatter(seed: &mut u64, low: f64, high: f64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    low + (high - low) * ((*seed >> 11) as f64 / (1u64 << 53) as f64)
}

#[test]
fn the_pairs_are_what_trying_every_pair_finds() {
    let mut seed = 5;
    for _ in 0..20 {
        let points: Vec<V2> = (0..30)
            .map(|_| {
                let x = scatter(&mut seed, -80.0, 80.0) as f32;
                V2::new(x, scatter(&mut seed, -80.0, 80.0) as f32)
            })
            .collect();
        let reaches: Vec<f64> = (0..30).map(|_| scatter(&mut seed, 1.0, 20.0)).collect();
        let mut every = vec![];
        for i in 0..30 {
            for j in i + 1..30 {
                let apart = points[i].distance_to(points[j]) as f64;
                if apart <= reaches[i] + reaches[j] + 2.0 {
                    every.push((i, j));
                }
            }
        }
        assert_eq!(pairs_near(&points, &reaches, 2.0), every);
    }
}

#[test]
fn the_gap_between_large_groups_is_exact_within_group_join() {
    let mut seed = 11;
    let mut within = 0;
    for trial in 0..20 {
        let mut b = built(&[line(1, 20, 20.0), line(2, 20, 20.0)]);
        for unit in 0..40 {
            let shift = if unit >= 20 { trial as f32 * 2.0 } else { 0.0 };
            let x = scatter(&mut seed, 0.0, 10.0) as f32 + shift;
            b.columns.position[unit] = V2::new(x, scatter(&mut seed, 0.0, 40.0) as f32);
        }
        let mut least = f64::INFINITY;
        for unit in 0..20 {
            for friend in 20..40 {
                let apart = b.columns.position[unit].distance_to(b.columns.position[friend]);
                least = minf(least, apart as f64 - 1.0);
            }
        }
        least = maxf(least, 0.0);
        let got = gap(&b.columns, &b.roster, 0, 1, &mut None, JOIN);
        if least <= JOIN {
            assert_eq!(got, least);
            within += 1;
        } else {
            assert!(got > JOIN);
        }
    }
    assert!((1..20).contains(&within));
}
