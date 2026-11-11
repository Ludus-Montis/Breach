//! The pair search's scenarios (spec 41 step 3): the flat grid, per-body sorts and the
//! neighbour list find the pairs UnitBodies._pairs names, in its order.

use super::*;
use crate::grid::cell_of;
use crate::parting::{framed, EPSILON, FLEEING, LOOSE};

/// The bodies' columns, kept so a test can borrow them as `Bodies`.
struct Crowd {
    points: Vec<V2>,
    flags: Vec<u8>,
    bases: Vec<V2>,
    radii: Vec<f64>,
    areas: Vec<f64>,
    squads: Vec<i32>,
    factions: Vec<i32>,
}

impl Crowd {
    /// `n` bodies packed tight: three squads in two factions, some loose, some fleeing, some
    /// on one spot, a few wider than a bucket (their pairs can reach past the next bucket).
    fn new(n: usize) -> Crowd {
        let mut seed: u32 = 777;
        let mut next = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let side = (n as f32).sqrt() * 0.6;
        let mut points: Vec<V2> = Vec::with_capacity(n);
        for k in 0..n {
            if k % 17 == 3 {
                points.push(points[k - 1]);
            } else {
                points.push(V2::new(next() * side - 7.0, next() * side - 3.0));
            }
        }
        let radii: Vec<f64> = (0..n)
            .map(|k| match k % 23 {
                0 => 1.5,
                _ if k % 5 == 0 => 1.0,
                _ => 0.5,
            })
            .collect();
        Crowd {
            points,
            flags: (0..n)
                .map(|k| match k % 7 {
                    0 | 4 => LOOSE,
                    5 => FLEEING,
                    _ => 0,
                })
                .collect(),
            bases: (0..n).map(|k| if k % 7 == 5 { V2::new(0.25, 0.0) } else { V2::ZERO }).collect(),
            areas: radii.iter().map(|r| (r * 2.0) * (r * 2.0)).collect(),
            radii,
            squads: (0..n).map(|k| (k % 3) as i32).collect(),
            factions: (0..n).map(|k| (k % 3 == 2) as i32).collect(),
        }
    }

    fn bodies(&self) -> Bodies<'_> {
        Bodies {
            points: self.points.clone(),
            nexts: self.points.clone(),
            flags: self.flags.clone(),
            bases: &self.bases,
            radii: &self.radii,
            areas: &self.areas,
            squads: &self.squads,
            factions: &self.factions,
        }
    }
}

/// UnitBodies._pairs as it reads, every pair tried: bodies in buckets next to each other,
/// overlapping, not in two squads' frames, sorted.
fn fresh(bodies: &Bodies, at: &[V2]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for i in 0..at.len() {
        for j in i + 1..at.len() {
            let (a, b) = (cell_of(at[i]), cell_of(at[j]));
            if (a.0 - b.0).abs() > 1 || (a.1 - b.1).abs() > 1 {
                continue;
            }
            if framed(bodies.flags[i])
                && framed(bodies.flags[j])
                && bodies.squads[i] != bodies.squads[j]
            {
                continue;
            }
            if bodies.radii[i] + bodies.radii[j] - at[j].sub(at[i]).length() as f64 > EPSILON {
                found.push((i, j));
            }
        }
    }
    found
}

#[test]
fn the_pairs_are_the_ones_a_search_of_every_pair_finds_in_its_order() {
    let crowd = Crowd::new(3000);
    let bodies = crowd.bodies();
    let expected = fresh(&bodies, &crowd.points);
    assert!(expected.len() > 3000, "the crowd should overlap a lot: {} pairs", expected.len());
    assert!(
        expected.iter().any(|&(i, j)| crowd.radii[i] + crowd.radii[j] > 2.0
            && crowd.points[j].sub(crowd.points[i]).length() > 2.0),
        "some pairs should reach past a bucket"
    );

    for pooled in [false, true] {
        let mut neighbours = Neighbours::default();
        assert_eq!(neighbours.pairs(&bodies, &crowd.points, pooled), expected, "pooled: {pooled}");
    }
}

#[test]
fn the_list_is_kept_while_bodies_move_less_than_half_the_skin_and_rebuilt_after() {
    let crowd = Crowd::new(1500);
    let mut bodies = crowd.bodies();
    let mut neighbours = Neighbours::default();
    let mut at = crowd.points.clone();
    neighbours.pairs(&bodies, &at, false);
    assert_eq!(neighbours.builds(), 1);

    // every body a little way, each its own way; some out of their frames
    let short = (SKIN / 2.0 * 0.8) as f32;
    for (k, p) in at.iter_mut().enumerate() {
        let way = (k as f32 * 0.37).sin_cos();
        *p = p.add(V2::new(way.0 * short, way.1 * short));
    }
    for k in (1..at.len()).step_by(11) {
        bodies.flags[k] |= LOOSE;
    }
    for pooled in [false, true] {
        assert_eq!(neighbours.pairs(&bodies, &at, pooled), fresh(&bodies, &at), "pooled: {pooled}");
    }
    assert_eq!(neighbours.builds(), 1, "nobody moved half the skin");

    // one body further: the list is built again before the pairs are read
    at[42] = at[42].add(V2::new(SKIN as f32, 0.0));
    assert_eq!(neighbours.pairs(&bodies, &at, false), fresh(&bodies, &at));
    assert_eq!(neighbours.builds(), 2);
}
