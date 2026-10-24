//! GroupSplit's scenarios (spec 30 round 3, part 4), on a squad built here: a line of four
//! grems facing east with its front at x = 40, on a route along y = 0 (their places are at
//! x = 39.5, y = -1.5 to 1.5, column 0 to 3).

use super::*;
use crate::ground::Walker;
use crate::roster::{Listed, Member};

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules { lost: 2.0, join: 2.0, seed: 7, cells_per_tile: 64.0 }
}

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

/// The line, unit k (id k + 1, store slot k, column k) standing at `at[k]`; its living in
/// the order `order` gives (indices of `at`).
fn built(at: &[V2], order: &[usize]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    squads.open(1);
    squads.position[0] = V2::new(40.0, 0.0);
    squads.heading[0] = 90.0;
    squads.width[0] = 4;
    squads.direction[0] = 1;
    for (k, &point) in at.iter().enumerate() {
        columns.grow();
        columns.position[k] = point;
        columns.column[k] = k as i32;
    }
    let living = order
        .iter()
        .map(|&k| Member {
            id: k as i64 + 1,
            slot: k,
            foot: (1, 1),
            preferred: 0,
            attack_range: 0,
            kind: 0,
            turn_rate: 360.0,
            backward_pace: 0.5,
            detection: 8.0,
            max_stamina: 100.0,
            walker: GREM,
        })
        .collect();
    let listed = Listed { squad: 0, id: 1, faction: 0, living, ..Listed::default() };
    Built { squads, columns, roster: Roster { listed: vec![listed] } }
}

fn candidate(b: &Built, waits: bool) -> Candidate {
    let count = b.roster.listed[0].living.len();
    Candidate {
        listed: 0,
        waits,
        route: Route::new(vec![V2::ZERO, V2::new(100.0, 0.0)], 0.0),
        taking: vec![false; count],
        offsets: vec![V2::ZERO; count],
    }
}

/// The ids of each split's units, in their order, with its front.
fn split_ids(b: &Built, candidates: &[Candidate]) -> Vec<(Vec<i64>, f64)> {
    let splits = step(&b.squads, &b.columns, &b.roster, candidates, &rules());
    let living = &b.roster.listed[0].living;
    splits.iter().map(|s| (s.members.iter().map(|&u| living[u].id).collect(), s.front)).collect()
}

fn placed(k: usize) -> V2 {
    V2::new(39.5, k as f32 - 1.5)
}

#[test]
fn units_fallen_far_behind_go_on_as_a_group_lined_up_across_by_where_they_stand() {
    let b = built(&[placed(0), placed(1), V2::new(20.0, 0.5), V2::new(21.0, 1.5)], &[0, 1, 2, 3]);
    // facing east, across runs north: the unit further south (larger y) lines up first
    assert_eq!(split_ids(&b, &[candidate(&b, false)]), vec![(vec![4, 3], 21.0 / 64.0)]);
}

#[test]
fn no_one_splits_off_if_no_man_is_left_behind() {
    let b = built(&[placed(0), placed(1), V2::new(20.0, 0.5), V2::new(21.0, 1.5)], &[0, 1, 2, 3]);
    assert!(split_ids(&b, &[candidate(&b, true)]).is_empty());
}

#[test]
fn no_one_splits_off_if_every_cluster_is_behind() {
    let behind: Vec<V2> = (0..4).map(|k| V2::new(20.0, k as f32 - 1.5)).collect();
    let b = built(&behind, &[0, 1, 2, 3]);
    assert!(split_ids(&b, &[candidate(&b, false)]).is_empty());
}

#[test]
fn a_cluster_with_one_unit_near_its_place_stays() {
    // unit 2 is further than walk_lost from its place but within reach of unit 1 at its own
    let b = built(&[placed(0), placed(1), V2::new(37.0, 0.5), placed(3)], &[0, 1, 2, 3]);
    assert!(split_ids(&b, &[candidate(&b, false)]).is_empty());
}

#[test]
fn units_loose_or_out_taking_are_not_looked_at() {
    let mut b =
        built(&[placed(0), placed(1), V2::new(20.0, 0.5), V2::new(21.0, 1.5)], &[0, 1, 2, 3]);
    let mut taking = candidate(&b, false);
    taking.taking[3] = true;
    b.squads.loose[0].push(3);
    assert!(split_ids(&b, &[taking]).is_empty());
}

#[test]
fn clusters_apart_split_in_their_draws_order() {
    let b = built(&[placed(0), placed(1), V2::new(20.0, 0.5), V2::new(10.0, 1.5)], &[0, 1, 2, 3]);
    let got = split_ids(&b, &[candidate(&b, false)]);
    let mut want = vec![(vec![3], 20.0 / 64.0), (vec![4], 10.0 / 64.0)];
    want.sort_by_key(|(ids, _)| contest::draw(7, ids[0]));
    assert_eq!(got, want);
}

#[test]
fn the_splits_are_the_same_whatever_the_list_order() {
    let at = [placed(0), V2::new(20.0, -0.5), V2::new(20.0, 0.5), V2::new(10.0, 1.5)];
    let forward = built(&at, &[0, 1, 2, 3]);
    let reversed = built(&at, &[3, 2, 1, 0]);
    assert_eq!(
        split_ids(&forward, &[candidate(&forward, false)]),
        split_ids(&reversed, &[candidate(&reversed, false)])
    );
}
