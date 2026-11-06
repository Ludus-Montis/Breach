//! The body-parting pass of `sim/skirmish/formation/unit_bodies.gd`, over plain arrays
//! (BodyField gathers them from its own state, in the bodies' draw order).
//!
//! Every operation mirrors the GDScript path's precision and order, so results are bit for
//! bit the same: `Vector2` maths in f32 (Godot's `real_t`), GDScript `float`s in f64, a
//! `Vector2 * float` narrowing the float to f32 first, the same summation order, no fused
//! multiply-add (Rust never contracts `a * b + c` on its own).

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use rayon::prelude::*;

pub mod pass;
mod serial;
mod sums;

pub use crate::field::{FLEEING, LOOSE};
use crate::maths::V2;
use crate::threads::Threads;

/// Cells a bucket of the pair search spans (UnitBodies.BUCKET).
const BUCKET: f32 = 2.0;
/// Overlaps shallower than this are left (UnitBodies.EPSILON).
const EPSILON: f64 = 0.001;

/// The body was pushed this step (set on the way out).
pub const MOVED: u8 = 4;

/// The bodies of one step: what moves (`points`, `nexts`, `flags`) and what doesn't.
pub struct Bodies<'a> {
    /// A fleeing body's offset, a loose body's point, a framed body's place.
    pub points: Vec<V2>,
    /// A loose body's next point (only read for loose bodies).
    pub nexts: Vec<V2>,
    pub flags: Vec<u8>,
    /// A fleeing body's point on its route (only read for fleeing bodies).
    pub bases: &'a [V2],
    pub radii: &'a [f64],
    /// Footprint areas: mass out of the frame.
    pub areas: &'a [f64],
    pub squads: &'a [i32],
    pub factions: &'a [i32],
}

/// The tuning the pass reads (BattleTuning bodies_passes, bodies_resist, bodies_brush).
pub struct Tuning {
    pub passes: i64,
    pub resist: f64,
    pub brush: f64,
}

/// What a step did: passes that moved something, pairs pushed, and the bodies that came
/// out of their frames in the order the GDScript path makes them loose.
#[derive(Default)]
pub struct Outcome {
    pub passes: i64,
    pub pairs: i64,
    pub loosened: Vec<i32>,
}

/// One pair's push: the move of each body (None: not moved), or "parts along a seeded way".
#[derive(Clone, Copy)]
enum Push {
    None,
    Moves(Option<V2>, Option<V2>),
    Coincident { depth: f64, share: f64 },
}

#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(*b as u64);
        }
    }
    fn write_i32(&mut self, v: i32) {
        self.write_u64(v as u32 as u64);
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

type Buckets = HashMap<(i32, i32), Vec<u32>, BuildHasherDefault<Fx>>;

#[inline]
fn framed(flags: u8) -> bool {
    flags & (LOOSE | FLEEING) == 0
}

/// Pushes apart every pair of bodies that overlap, `tuning.passes` times or until none do.
/// `way(i, j)` gives the seeded way two bodies lying on each other part along (it calls
/// back into GDScript, so is only ever called on this thread). With `threaded` on for the
/// bodies' count (`Threads::on`), pairs are found, pushes weighed and each body's pushes
/// summed on Rayon's pool, from the pass's snapshot, in pair order (`sums`).
pub fn part(
    bodies: &mut Bodies,
    tuning: &Tuning,
    threaded: Threads,
    way: &mut dyn FnMut(usize, usize) -> V2,
) -> Outcome {
    let count = bodies.points.len();
    let pooled = threaded.on(count);
    let mut out = Outcome::default();
    let mut serial = serial::Serial::new(count);
    for _ in 0..tuning.passes {
        let at = positions(bodies);
        let pairs = pairs(bodies, &at, pooled);
        out.pairs += pairs.len() as i64;
        let moved = if pooled {
            let mut pushes: Vec<Push> =
                pairs.par_iter().map(|&(i, j)| push(bodies, &at, tuning, i, j)).collect();
            sums::resolve(&pairs, &mut pushes, way);
            sums::summed(bodies, &pairs, &pushes)
        } else {
            let pushes = pairs.iter().map(|&(i, j)| push(bodies, &at, tuning, i, j));
            serial.summed(bodies, &pairs, pushes, way)
        };
        if moved.is_empty() {
            break;
        }
        out.passes += 1;
        for (index, push, loosens) in moved {
            apply(bodies, index, push, loosens, &mut out.loosened);
        }
    }
    out
}

/// A coincident pair's moves along the seeded way `w`.
fn moves_along(w: V2, depth: f64, share: f64) -> Push {
    let one = (share > 0.0).then(|| w.scale(depth).scale(share));
    let two = (share < 1.0).then(|| w.scale(depth).scale(1.0 - share));
    Push::Moves(one, two)
}

/// Where every body stands (UnitBodies.at).
fn positions(bodies: &Bodies) -> Vec<V2> {
    (0..bodies.points.len())
        .map(|i| {
            if bodies.flags[i] & FLEEING != 0 {
                bodies.bases[i].add(bodies.points[i])
            } else {
                bodies.points[i]
            }
        })
        .collect()
}

/// [(i, j), ...] sorted, for bodies that overlap, not in two squads' frames
/// (UnitBodies._pairs).
fn pairs(bodies: &Bodies, at: &[V2], threaded: bool) -> Vec<(usize, usize)> {
    let count = at.len();
    let homes: Vec<(i32, i32)> =
        at.iter().map(|p| ((p.x / BUCKET).floor() as i32, (p.y / BUCKET).floor() as i32)).collect();
    let mut buckets = Buckets::default();
    for (index, home) in homes.iter().enumerate() {
        buckets.entry(*home).or_default().push(index as u32);
    }
    let found_from = |index: usize| -> Vec<u64> {
        let mut found = Vec::new();
        let home = homes[index];
        for dy in -1..=1 {
            for dx in -1..=1 {
                let Some(bucket) = buckets.get(&(home.0 + dx, home.1 + dy)) else {
                    continue;
                };
                for &other in bucket {
                    let other = other as usize;
                    if other <= index {
                        continue;
                    }
                    let both_framed = framed(bodies.flags[index]) && framed(bodies.flags[other]);
                    if both_framed && bodies.squads[index] != bodies.squads[other] {
                        continue; // two squads' frames keep them apart
                    }
                    let apart = at[other].sub(at[index]);
                    if bodies.radii[index] + bodies.radii[other] - apart.length() as f64 > EPSILON {
                        found.push((index * count + other) as u64);
                    }
                }
            }
        }
        found
    };
    let mut found: Vec<u64> = if threaded {
        (0..count).into_par_iter().flat_map_iter(found_from).collect()
    } else {
        (0..count).flat_map(found_from).collect()
    };
    if threaded {
        found.par_sort_unstable();
    } else {
        found.sort_unstable();
    }
    let count = count as u64;
    found.into_iter().map(|k| ((k / count) as usize, (k % count) as usize)).collect()
}

/// The pair's push (UnitBodies._push): each body out along the line between them - friends
/// by the other's share of their mass, foes half each.
fn push(bodies: &Bodies, at: &[V2], tuning: &Tuning, a: usize, b: usize) -> Push {
    let apart = at[b].sub(at[a]);
    let depth = bodies.radii[a] + bodies.radii[b] - apart.length() as f64;
    if depth <= EPSILON {
        return Push::None;
    }
    let mut share = 0.5;
    if bodies.factions[a] == bodies.factions[b] {
        share = mass(bodies, tuning, b) / (mass(bodies, tuning, a) + mass(bodies, tuning, b));
        if keeps(bodies, tuning, a, b, depth) {
            share = 0.0;
        } else if keeps(bodies, tuning, b, a, depth) {
            share = 1.0;
        }
    }
    if (apart.length() as f64) < EPSILON {
        return Push::Coincident { depth, share };
    }
    let way = apart.normalized();
    let one = (share > 0.0).then(|| way.scale(depth).scale(share));
    let two = (share < 1.0).then(|| way.scale(depth).scale(1.0 - share));
    Push::Moves(one, two)
}

/// UnitBodies._keeps.
fn keeps(bodies: &Bodies, tuning: &Tuning, keeper: usize, other: usize, depth: f64) -> bool {
    if !framed(bodies.flags[keeper]) || bodies.flags[other] & LOOSE == 0 {
        return false;
    }
    bodies.squads[keeper] != bodies.squads[other] || depth < tuning.brush
}

/// UnitBodies._mass.
fn mass(bodies: &Bodies, tuning: &Tuning, index: usize) -> f64 {
    let area = bodies.areas[index];
    if framed(bodies.flags[index]) {
        area * tuning.resist
    } else {
        area
    }
}

/// UnitBodies._move: a router's offset, a loose unit's point, a framed unit made loose if
/// a body out of its frame pushed it (`loosens`), else its point in its frame.
fn apply(bodies: &mut Bodies, index: usize, push: V2, loosens: bool, loosened: &mut Vec<i32>) {
    let flags = bodies.flags[index];
    bodies.flags[index] |= MOVED;
    if flags & FLEEING != 0 || (flags & LOOSE == 0 && !loosens) {
        bodies.points[index] = bodies.points[index].add(push);
        return;
    }
    if flags & LOOSE == 0 {
        bodies.flags[index] |= LOOSE;
        bodies.nexts[index] = bodies.points[index];
        loosened.push(index as i32);
    }
    let resting = bodies.nexts[index] == bodies.points[index];
    bodies.points[index] = bodies.points[index].add(push);
    if resting {
        bodies.nexts[index] = bodies.points[index];
    }
}

#[cfg(test)]
mod tests;
