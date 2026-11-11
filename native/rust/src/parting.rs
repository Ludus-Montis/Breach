//! The body-parting pass of `sim/skirmish/formation/unit_bodies.gd`, over plain arrays
//! (BodyField gathers them from its own state, in the bodies' draw order).
//!
//! Every operation mirrors the GDScript path's precision and order, so results are bit for
//! bit the same: `Vector2` maths in f32 (Godot's `real_t`), GDScript `float`s in f64, a
//! `Vector2 * float` narrowing the float to f32 first, the same summation order, no fused
//! multiply-add (Rust never contracts `a * b + c` on its own).

use rayon::prelude::*;

pub mod pairs;
pub mod pass;

pub use crate::field::{FLEEING, LOOSE};
use crate::laps::split;
use crate::maths::V2;
use crate::threads::Threads;

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
enum Push {
    None,
    Moves(Option<V2>, Option<V2>),
    Coincident { depth: f64, share: f64 },
}

/// What parting keeps on the field between ticks (`buffers.rs`, spec 41 step 4): a pass's
/// positions, pairs, pushes and sums, and the tick's neighbour list, each cleared and filled
/// afresh by every parting.
#[derive(Default)]
pub struct Buffers {
    moves: Vec<V2>,
    has_move: Vec<bool>,
    loosens: Vec<bool>,
    order: Vec<usize>,
    at: Vec<V2>,
    pairs: Vec<(usize, usize)>,
    pushes: Vec<Push>,
    neighbours: pairs::Neighbours,
}

impl Buffers {
    /// True once a parting has kept something in them.
    pub fn warm(&self) -> bool {
        self.moves.capacity() > 0 || self.neighbours.warm()
    }
}

#[inline]
fn framed(flags: u8) -> bool {
    flags & (LOOSE | FLEEING) == 0
}

/// Pushes apart every pair of bodies that overlap, `tuning.passes` times or until none do.
/// `way(i, j)` gives the seeded way two bodies lying on each other part along (it calls
/// back into GDScript, so is only ever called on this thread). With `threaded` on for the
/// bodies' count (`Threads::on`), pairs are found and pushes weighed on Rayon's pool, from
/// the pass's snapshot, and applied in pair order.
#[cfg(test)]
pub fn part(
    bodies: &mut Bodies,
    tuning: &Tuning,
    threaded: Threads,
    way: &mut dyn FnMut(usize, usize) -> V2,
) -> Outcome {
    part_in(bodies, tuning, threaded, way, &mut Buffers::default())
}

/// `part`, in buffers kept from an earlier parting (or new ones): the same parting either way.
pub fn part_in(
    bodies: &mut Bodies,
    tuning: &Tuning,
    threaded: Threads,
    way: &mut dyn FnMut(usize, usize) -> V2,
    kept: &mut Buffers,
) -> Outcome {
    let count = bodies.points.len();
    let pooled = threaded.on(count);
    let mut out = Outcome::default();
    let Buffers { moves, has_move, loosens, order, at, pairs, pushes, neighbours } = kept;
    moves.clear();
    moves.resize(count, V2::ZERO);
    has_move.clear();
    has_move.resize(count, false);
    // pushed by a body out of its frame: one in its frame leaves it (UnitBodies._move)
    loosens.clear();
    loosens.resize(count, false);
    neighbours.clear();
    for _ in 0..tuning.passes {
        positions(bodies, at);
        neighbours.pairs_into(bodies, at, pooled, pairs);
        out.pairs += pairs.len() as i64;
        split("bodies: pair search");
        pushes.clear();
        if pooled {
            pushes.par_extend(pairs.par_iter().map(|&(i, j)| push(bodies, at, tuning, i, j)));
        } else {
            pushes.extend(pairs.iter().map(|&(i, j)| push(bodies, at, tuning, i, j)));
        }
        split("bodies: pushes");
        order.clear();
        for (&(i, j), p) in pairs.iter().zip(pushes.drain(..)) {
            let loosening = !(framed(bodies.flags[i]) && framed(bodies.flags[j]));
            let (one, two) = match p {
                Push::None => continue,
                Push::Moves(one, two) => (one, two),
                Push::Coincident { depth, share } => {
                    let w = way(i, j);
                    let one = (share > 0.0).then(|| w.scale(depth).scale(share));
                    let two = (share < 1.0).then(|| w.scale(depth).scale(1.0 - share));
                    (one, two)
                }
            };
            if let Some(v) = one {
                accumulate(moves, has_move, order, i, v, true);
                loosens[i] |= loosening;
            }
            if let Some(v) = two {
                accumulate(moves, has_move, order, j, v, false);
                loosens[j] |= loosening;
            }
        }
        if order.is_empty() {
            split("bodies: sums and applying");
            break;
        }
        out.passes += 1;
        for &index in order.iter() {
            apply(bodies, index, moves[index], loosens[index], &mut out.loosened);
            moves[index] = V2::ZERO;
            has_move[index] = false;
            loosens[index] = false;
        }
        split("bodies: sums and applying");
    }
    out
}

/// `moves[i] = moves.get(i, Vector2.ZERO) -/+ v`, noting the order keys first appear.
#[inline]
fn accumulate(
    moves: &mut [V2],
    has_move: &mut [bool],
    order: &mut Vec<usize>,
    index: usize,
    v: V2,
    minus: bool,
) {
    if !has_move[index] {
        has_move[index] = true;
        order.push(index);
    }
    moves[index] = if minus { moves[index].sub(v) } else { moves[index].add(v) };
}

/// Where every body stands (UnitBodies.at), into `at`.
fn positions(bodies: &Bodies, at: &mut Vec<V2>) {
    at.clear();
    at.extend((0..bodies.points.len()).map(|i| {
        if bodies.flags[i] & FLEEING != 0 {
            bodies.bases[i].add(bodies.points[i])
        } else {
            bodies.points[i]
        }
    }));
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
