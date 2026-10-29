//! FormationShuffle's swaps under way (Decisions 46, 49 and 51), on squads built here: units
//! given by (rank, column), each one cell, unit k (id k + 1) in store slot k, all preferring
//! the front. Which moves start, and the scenarios through a battle, stay gdUnit's and
//! `march::tests`'.

use super::*;

struct Built {
    squads: Squads,
    columns: Columns,
}

/// A squad of `width` at front 5.0 facing `direction` 1, and units at `places` in store slots
/// from 0 in the order given.
fn built(width: i32, places: &[(i32, i32)]) -> Built {
    let mut b = Built { squads: Squads::default(), columns: Columns::default() };
    b.squads.open(1);
    b.squads.width[0] = width;
    b.squads.direction[0] = 1;
    b.squads.front_distance[0] = 5.0;
    for (k, &(rank, column)) in places.iter().enumerate() {
        b.columns.grow();
        b.columns.rank[k] = rank;
        b.columns.column[k] = column;
    }
    b
}

fn closers(slots: &[usize]) -> Vec<Closer> {
    slots
        .iter()
        .map(|&k| Closer { id: k as i64 + 1, slot: k, foot: (1, 1), preferred: 0 })
        .collect()
}

fn unit(slot: usize, to: (i32, i32)) -> SwapUnit {
    SwapUnit { slot, alive: true, to, foot: (1, 1) }
}

fn swap(ticks: i64, units: &[SwapUnit]) -> Swap {
    Swap { ticks, units: units.to_vec() }
}

fn places(b: &Built) -> Vec<(i32, i32)> {
    (0..b.columns.rank.len()).map(|k| (b.columns.rank[k], b.columns.column[k])).collect()
}

fn run(b: &mut Built, living: &[usize], swaps: &[Swap]) -> Ends {
    let all: Vec<usize> = (0..b.columns.rank.len()).collect();
    end(&mut b.squads, &mut b.columns, 0, &closers(living), &all, swaps)
}

#[test]
fn a_swap_with_time_left_counts_down_and_nobody_moves() {
    let mut b = built(2, &[(0, 0), (1, 1)]);
    let ends = run(&mut b, &[0, 1], &[swap(3, &[unit(1, (0, 1))])]);
    assert_eq!(ends, Ends { kept: vec![(0, 2)], left: 0, ended: vec![] });
    assert_eq!(places(&b), vec![(0, 0), (1, 1)]);
}

#[test]
fn a_swap_ending_puts_its_units_in_place_and_the_gap_it_left_closes() {
    let mut b = built(1, &[(1, 0), (2, 0)]);
    let ends = run(&mut b, &[0, 1], &[swap(1, &[unit(0, (0, 0))])]);
    assert_eq!(ends, Ends { kept: vec![], left: 0, ended: vec![(0, vec![(1, 1)])] });
    assert_eq!(places(&b), vec![(0, 0), (1, 0)]);
}

#[test]
fn a_swap_one_of_whose_units_has_fallen_is_cancelled_where_they_stand() {
    let mut b = built(2, &[(1, 0), (0, 0)]);
    let fallen = SwapUnit { alive: false, ..unit(1, (1, 0)) };
    let ends = run(&mut b, &[0], &[swap(1, &[unit(0, (0, 0)), fallen])]);
    assert_eq!(ends, Ends::default());
    assert_eq!(places(&b), vec![(1, 0), (0, 0)]);
}

#[test]
fn ending_outside_the_columns_widens_the_squad_and_renumbers_every_place() {
    // u0 and u1 hold the front; u2 moves to the front left of the columns; u3 has fallen;
    // u4 is moving back a rank. The last swap ends first and widens the squad by one on
    // the left; the other ends after, at its place renumbered, and steps up into the gap.
    let mut b = built(2, &[(0, 0), (0, 1), (1, 0), (2, 1), (1, 1)]);
    let swaps = [swap(1, &[unit(4, (2, 1))]), swap(1, &[unit(2, (0, -1))])];
    let ends = run(&mut b, &[0, 1, 2, 4], &swaps);
    assert_eq!(ends, Ends { kept: vec![], left: -1, ended: vec![(1, vec![]), (0, vec![(3, 1)])] });
    assert_eq!(places(&b), vec![(0, 1), (0, 2), (0, 0), (2, 2), (1, 2)]);
    assert_eq!(b.squads.width[0], 3);
    assert_eq!(b.squads.centre_shift[0], -0.5);
}

#[test]
fn a_widening_on_the_right_keeps_the_columns_and_a_swap_under_way_counts_down() {
    let mut b = built(1, &[(0, 0), (1, 0), (2, 0)]);
    let swaps = [swap(4, &[unit(2, (1, 0))]), swap(1, &[unit(1, (0, 1))])];
    let ends = run(&mut b, &[0, 1, 2], &swaps);
    assert_eq!(ends, Ends { kept: vec![(0, 3)], left: 0, ended: vec![(1, vec![])] });
    assert_eq!(b.squads.width[0], 2);
    assert_eq!(b.squads.centre_shift[0], 0.5);
}

// The swaps the crate holds (spec 39 step 8b4-v).

fn held_unit(slot: usize, from: (i32, i32), to: (i32, i32)) -> HeldUnit {
    HeldUnit { id: slot as i64 + 1, slot, to, from, foot: (1, 1) }
}

fn held(ticks: i64, total: i64, units: &[HeldUnit], passed: &[i64]) -> Held {
    Held { ticks, total, units: units.to_vec(), passed: passed.to_vec() }
}

fn run_held(b: &mut Built, living: &[usize]) -> Vec<Ended> {
    let all: Vec<usize> = (0..b.columns.rank.len()).collect();
    end_held(&mut b.squads, &mut b.columns, 0, &closers(living), &all)
}

#[test]
fn a_held_swap_with_time_left_counts_down_and_stays() {
    let mut b = built(2, &[(0, 0), (1, 1)]);
    b.squads.swaps[0] = vec![held(3, 5, &[held_unit(1, (1, 1), (0, 1))], &[])];
    assert_eq!(run_held(&mut b, &[0, 1]), vec![]);
    assert_eq!(b.squads.swaps[0], vec![held(2, 5, &[held_unit(1, (1, 1), (0, 1))], &[])]);
}

#[test]
fn a_held_swap_ending_reports_its_mover_and_passed_and_is_dropped() {
    let mut b = built(2, &[(0, 0), (1, 1)]);
    b.squads.swaps[0] = vec![held(1, 4, &[held_unit(1, (1, 1), (0, 1))], &[1])];
    let ended = run_held(&mut b, &[0, 1]);
    assert_eq!(ended, vec![Ended { mover: 2, passed: vec![1], stepped: vec![] }]);
    assert!(b.squads.swaps[0].is_empty());
    assert_eq!(places(&b), vec![(0, 0), (0, 1)]);
}

#[test]
fn a_held_swap_one_of_whose_units_is_dead_is_cancelled() {
    let mut b = built(2, &[(0, 0), (1, 1)]);
    b.columns.state[1] = DEAD;
    b.squads.swaps[0] = vec![held(1, 4, &[held_unit(1, (1, 1), (0, 1))], &[])];
    assert_eq!(run_held(&mut b, &[0]), vec![]);
    assert!(b.squads.swaps[0].is_empty());
    assert_eq!(places(&b), vec![(0, 0), (1, 1)], "nobody moved");
}

#[test]
fn moves_begun_are_held_and_claim_their_places() {
    let mut b = built(2, &[(0, 0), (1, 1), (2, 0)]);
    let plan = crate::march::shuffle::Plan {
        mover: 1,
        passed: vec![0],
        to: vec![(1, (0, 1)), (0, (1, 1))],
        distance: 2,
    };
    let started = [Started { plan, ticks: 4 }];
    let living = [(11, 0), (12, 1), (13, 2)];
    let places = [(0, 0), (1, 1), (2, 0)];
    begin(&mut b.squads, 0, &started, &living, &places, &[(1, 1), (2, 1), (1, 1)]);
    let mover = HeldUnit { id: 12, slot: 1, to: (0, 1), from: (1, 1), foot: (2, 1) };
    let other = HeldUnit { id: 11, slot: 0, to: (1, 1), from: (0, 0), foot: (1, 1) };
    assert_eq!(b.squads.swaps[0], vec![held(4, 4, &[mover, other], &[11])]);
    let (claimed, busy) = claims(&b.squads, 0, &[2, 1]);
    assert_eq!(claimed, vec![((0, 1), 2, 1), ((1, 1), 1, 1)]);
    assert_eq!(busy, vec![1], "slot 1 is second of those living; slot 0 isn't listed");
    let moving: Vec<(usize, (i32, i32))> =
        super::moving(&b.squads, 0).iter().map(|m| (m.slot, m.to)).collect();
    assert_eq!(moving, vec![(1, (0, 1)), (0, (1, 1))]);
}

#[test]
fn a_unit_s_offset_is_how_far_it_is_through_its_first_swap() {
    let mut b = built(2, &[(0, 0), (1, 1)]);
    b.squads.swaps[0] = vec![
        held(1, 4, &[held_unit(1, (1, 1), (0, 0))], &[]),
        held(2, 4, &[held_unit(1, (1, 1), (0, 1)), held_unit(0, (0, 0), (0, 1))], &[]),
    ];
    assert_eq!(offsets(&b.squads, 0), vec![(2, 0.75, -0.75), (1, 0.0, 0.5)]);
}
