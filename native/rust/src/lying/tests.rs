//! Bodies on the ground (Decision 121, spec 30 round 3): the step a walker takes over them,
//! and a marching front's share of its pace, on bodies and squads built here. The tuning is
//! the shipped one: bodies_ground_drag 1, bodies_ground_block 8.

use super::*;
use crate::ground::Walker;
use crate::reach;
use crate::roster::Member;

const DRAG: f64 = 1.0;
const BLOCK: f64 = 8.0;

/// Bodies as (where it lies, its footprint's side), indexed.
struct Bodies {
    ats: Vec<V2>,
    radii: Vec<f64>,
    areas: Vec<f64>,
}

fn bodies(all: &[(V2, i64)]) -> Bodies {
    Bodies {
        ats: all.iter().map(|b| b.0).collect(),
        radii: all.iter().map(|b| reach::radius(b.1, b.1)).collect(),
        areas: all.iter().map(|b| (b.1 * b.1) as f64).collect(),
    }
}

impl Bodies {
    fn lying(&self) -> Lying<'_> {
        Lying::new(&self.ats, &self.radii, &self.areas)
    }
}

/// A grem (one cell square) stepping onto `step`.
fn grem_steps(lying: &Lying, step: V2) -> f64 {
    lying.underfoot(reach::radius(1, 1), 1.0, step, DRAG, BLOCK)
}

#[test]
fn a_body_underfoot_slows_by_its_weight_against_the_walkers() {
    let grem = bodies(&[(V2::new(0.5, 0.0), 1)]);
    let brute = bodies(&[(V2::new(1.0, 0.0), 2)]);
    assert_eq!(grem_steps(&grem.lying(), V2::new(0.5, 0.0)), 1.0 / (1.0 + DRAG));
    assert_eq!(grem_steps(&brute.lying(), V2::new(0.5, 0.0)), 1.0 / (1.0 + 4.0 * DRAG));
    assert_eq!(grem_steps(&grem.lying(), V2::new(5.0, 0.0)), 1.0);
}

#[test]
fn a_body_too_heavy_blocks_the_step() {
    let huge = bodies(&[(V2::new(0.5, 0.0), 3)]);
    assert_eq!(grem_steps(&huge.lying(), V2::new(0.5, 0.0)), 0.0);
}

/// A scattered field's points: a fixed sequence over 40 cells square.
fn scattered(count: usize, seed: u64) -> Vec<(V2, i64)> {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as f32 / (1u64 << 31) as f32
    };
    (0..count)
        .map(|_| {
            let at = V2::new(next() * 40.0, next() * 40.0);
            (at, if next() < 0.2 { 2 } else { 1 })
        })
        .collect()
}

#[test]
fn the_bodies_underfoot_are_those_a_look_at_all_finds() {
    let field = bodies(&scattered(70, 5));
    let lying = field.lying();
    for (step, side) in scattered(200, 9) {
        let (own, area) = (reach::radius(side, side), (side * side) as f64);
        let mut worst = 1.0f64;
        for i in 0..field.ats.len() {
            if (field.ats[i].distance_to(step) as f64) >= own + field.radii[i] {
                continue;
            }
            let weight = field.areas[i] / area;
            worst = if weight >= BLOCK { 0.0 } else { worst.min(1.0 / (1.0 + weight * DRAG)) };
            if worst == 0.0 {
                break;
            }
        }
        assert_eq!(lying.underfoot(own, area, step, DRAG, BLOCK), worst);
    }
}

fn member(slot: usize) -> Member {
    Member {
        id: slot as i64 + 1,
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

/// A squad marching east (heading 90), its units (rank, where it stands) in that order.
fn marching(units: &[(i32, V2)]) -> (Squads, Columns, Listed) {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    squads.open(1);
    squads.heading[0] = 90.0;
    let mut living = Vec::new();
    for (slot, &(rank, at)) in units.iter().enumerate() {
        columns.grow();
        columns.rank[slot] = rank;
        columns.position[slot] = at;
        living.push(member(slot));
    }
    (squads, columns, Listed { squad: 0, id: 1, living, ..Listed::default() })
}

fn drag_over(units: &[(i32, V2)], lying: &[(V2, i64)]) -> f64 {
    let (squads, columns, listed) = marching(units);
    let field = bodies(lying);
    field.lying().drag(&squads, &columns, &listed, DRAG, BLOCK)
}

#[test]
fn a_body_its_own_weight_halves_its_pace_and_a_great_one_blocks_it() {
    let front = [(0, V2::ZERO)];
    assert_eq!(drag_over(&front, &[(V2::new(0.8, 0.0), 1)]), 0.5);
    assert_eq!(drag_over(&front, &[(V2::new(2.0, 0.0), 4)]), 0.0);
    assert_eq!(drag_over(&front, &[(V2::new(0.5, 3.0), 1)]), 1.0); // aside
    assert_eq!(drag_over(&front, &[]), 1.0);
}

#[test]
fn only_the_front_rank_steps_over_them_and_the_worst_step_counts() {
    let body = [(V2::new(0.8, 0.0), 1)];
    assert_eq!(drag_over(&[(1, V2::ZERO), (0, V2::new(0.0, 5.0))], &body), 1.0);
    let brute = (V2::new(0.8, 5.0), 2);
    let front = [(0, V2::ZERO), (0, V2::new(0.0, 5.0))];
    assert_eq!(drag_over(&front, &[body[0], brute]), 1.0 / (1.0 + 4.0 * DRAG));
    let reversed = [front[1], front[0]];
    assert_eq!(drag_over(&reversed, &[brute, body[0]]), 1.0 / (1.0 + 4.0 * DRAG));
}
