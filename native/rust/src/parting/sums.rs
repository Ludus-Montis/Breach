//! Each body's sum of its pushes in a pass, worked out on Rayon's pool in the order the serial
//! pass adds them (spec 39 step 10f), so the result is bit for bit the serial one.
//!
//! The serial pass walks the pairs in order and, for each, takes the first body's push off
//! its sum and adds the second's, noting each body when it is first pushed; the bodies are
//! then moved in that order. Here every push becomes a key, `body << 40 | pair << 1 | side`,
//! sorted on the pool: each body's keys then run together in pair order, and the first of
//! them is where the serial pass first met it. Each body's run is summed apart, from zero in
//! that order, and the sums are put in the order of their first keys.

use rayon::prelude::*;

use super::{framed, Bodies, Push};
use crate::maths::V2;

/// Bits of a key below the body: the pair and its side.
const PAIR_BITS: u32 = 40;
const PAIR_MASK: u64 = (1 << PAIR_BITS) - 1;

/// One body's move in a pass: its index, its summed push, and whether a body out of its frame
/// pushed it.
pub(super) type Moved = (usize, V2, bool);

/// The seeded way for pairs lying on one spot, asked on this thread in pair order (`way`
/// calls back into GDScript), each such push turned into the moves it gives.
pub(super) fn resolve(
    pairs: &[(usize, usize)],
    pushes: &mut [Push],
    way: &mut dyn FnMut(usize, usize) -> V2,
) {
    let coincident: Vec<usize> = pushes
        .par_iter()
        .enumerate()
        .filter(|(_, p)| matches!(p, Push::Coincident { .. }))
        .map(|(k, _)| k)
        .collect();
    for k in coincident {
        let (i, j) = pairs[k];
        if let Push::Coincident { depth, share } = pushes[k] {
            pushes[k] = super::moves_along(way(i, j), depth, share);
        }
    }
}

/// Every pushed body's move, in the order the serial pass first pushes each. `pushes` holds
/// no `Coincident` (see `resolve`).
pub(super) fn summed(bodies: &Bodies, pairs: &[(usize, usize)], pushes: &[Push]) -> Vec<Moved> {
    let mut keys: Vec<u64> = pushes
        .par_iter()
        .enumerate()
        .flat_map_iter(|(k, p)| {
            let (i, j) = pairs[k];
            let pair = (k as u64) << 1;
            let (one, two) = match p {
                Push::Moves(one, two) => (one.is_some(), two.is_some()),
                _ => (false, false),
            };
            let first = one.then_some(((i as u64) << PAIR_BITS) | pair);
            let second = two.then_some(((j as u64) << PAIR_BITS) | pair | 1);
            first.into_iter().chain(second)
        })
        .collect();
    keys.par_sort_unstable();
    let starts: Vec<usize> = (0..keys.len())
        .into_par_iter()
        .filter(|&e| e == 0 || keys[e] >> PAIR_BITS != keys[e - 1] >> PAIR_BITS)
        .collect();
    let mut moved: Vec<(u64, Moved)> = starts
        .par_iter()
        .enumerate()
        .map(|(run, &from)| {
            let to = starts.get(run + 1).copied().unwrap_or(keys.len());
            let run = &keys[from..to];
            ((run[0] & PAIR_MASK), sum(bodies, pairs, pushes, run))
        })
        .collect();
    moved.par_sort_unstable_by_key(|&(first, _)| first);
    moved.into_iter().map(|(_, m)| m).collect()
}

/// One body's run of keys, summed from zero in pair order as `accumulate` does.
fn sum(bodies: &Bodies, pairs: &[(usize, usize)], pushes: &[Push], run: &[u64]) -> Moved {
    let index = (run[0] >> PAIR_BITS) as usize;
    let mut total = V2::ZERO;
    let mut loosens = false;
    for &key in run {
        let k = ((key & PAIR_MASK) >> 1) as usize;
        let (i, j) = pairs[k];
        loosens |= !(framed(bodies.flags[i]) && framed(bodies.flags[j]));
        if let Push::Moves(one, two) = pushes[k] {
            total = match (key & 1 == 0, one, two) {
                (true, Some(v), _) => total.sub(v),
                (false, _, Some(v)) => total.add(v),
                _ => total,
            };
        }
    }
    (index, total, loosens)
}
