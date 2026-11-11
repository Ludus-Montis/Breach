//! Parting's pair search (UnitBodies._pairs, spec 41 step 3): the bodies in buckets next to
//! each other that overlap, not in two squads' frames, as `(i, j)` with `i < j`, sorted.
//!
//! A tick's passes share one neighbour list (a Verlet list): the pairs within their radii
//! plus `SKIN` of each other when it was built, found through `grid::Grid` (its cells are
//! UnitBodies.BUCKET across), each body's partners sorted, so the list is in pair order with
//! no global sort. A pass reads the list in order and keeps the pairs UnitBodies._pairs would
//! find now. The list is built again before a pass whenever a body has moved more than half
//! the skin since it was built, so it always holds every pair that overlaps: the pairs, and
//! their order, are a fresh search's.

#[cfg(test)]
mod tests;

use rayon::prelude::*;

use super::{framed, Bodies, EPSILON};
use crate::grid::{cell_of, Grid, MARGIN};
use crate::maths::V2;

/// Cells added to the overlap test when the list is built (the skin). Wider keeps the list
/// through more of a tick's movement, narrower lists fewer pairs; it changes speed only.
pub const SKIN: f64 = 0.25;
/// Bodies one buffer of the list is filled for (the list's allocations are one per chunk).
const CHUNK: usize = 1024;

/// A tick's neighbour list: where the bodies stood when it was built, and the pairs listed.
#[derive(Default)]
pub struct Neighbours {
    built: Vec<V2>,
    listed: Vec<(u32, u32)>,
    builds: usize,
}

impl Neighbours {
    /// The pairs that overlap at `at`, in pair order, the list built first if it is stale.
    pub fn pairs(&mut self, bodies: &Bodies, at: &[V2], pooled: bool) -> Vec<(usize, usize)> {
        if self.stale(at) {
            self.listed = listed(bodies, at, pooled);
            self.built = at.to_vec();
            self.builds += 1;
        }
        let homes: Vec<(i32, i32)> = at.iter().map(|&p| cell_of(p)).collect();
        let keeps = |&&(i, j): &&(u32, u32)| overlap(bodies, at, &homes, i as usize, j as usize);
        let found = |&(i, j): &(u32, u32)| (i as usize, j as usize);
        if pooled {
            self.listed.par_iter().filter(keeps).map(found).collect()
        } else {
            self.listed.iter().filter(keeps).map(found).collect()
        }
    }

    /// How many times the list was built.
    #[cfg(test)]
    pub fn builds(&self) -> usize {
        self.builds
    }

    /// Whether a body has moved more than half the skin (less a margin far above float
    /// rounding) since the list was built, or none was.
    fn stale(&self, at: &[V2]) -> bool {
        let limit = SKIN / 2.0 - MARGIN;
        self.built.len() != at.len()
            || self.built.iter().zip(at).any(|(&was, &now)| now.sub(was).length() as f64 > limit)
    }
}

/// UnitBodies._pairs' test for a listed pair: in buckets next to each other, not in two
/// squads' frames, overlapping.
#[inline]
fn overlap(bodies: &Bodies, at: &[V2], homes: &[(i32, i32)], i: usize, j: usize) -> bool {
    let (a, b) = (homes[i], homes[j]);
    if (a.0 - b.0).abs() > 1 || (a.1 - b.1).abs() > 1 {
        return false;
    }
    if framed(bodies.flags[i]) && framed(bodies.flags[j]) && bodies.squads[i] != bodies.squads[j] {
        return false; // two squads' frames keep them apart
    }
    bodies.radii[i] + bodies.radii[j] - at[j].sub(at[i]).length() as f64 > EPSILON
}

/// Every pair `(i, j)`, `i < j`, standing within their radii plus the skin of each other, in
/// pair order: each chunk of bodies fills one buffer, each body's partners sorted.
fn listed(bodies: &Bodies, at: &[V2], pooled: bool) -> Vec<(u32, u32)> {
    let grid = Grid::build(at);
    let widest = bodies.radii.iter().copied().fold(0.0, f64::max);
    let fill = |chunk: usize| -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for i in chunk * CHUNK..((chunk + 1) * CHUNK).min(at.len()) {
            let from = out.len();
            let reach = bodies.radii[i] + widest + SKIN + MARGIN;
            grid.near(at[i], reach, |j| {
                let j = j as usize;
                if j > i
                    && (at[j].sub(at[i]).length() as f64) < bodies.radii[i] + bodies.radii[j] + SKIN
                {
                    out.push((i as u32, j as u32));
                }
                true
            });
            out[from..].sort_unstable_by_key(|&(_, j)| j);
        }
        out
    };
    let chunks = at.len().div_ceil(CHUNK);
    let filled: Vec<Vec<(u32, u32)>> = if pooled {
        (0..chunks).into_par_iter().map(fill).collect()
    } else {
        (0..chunks).map(fill).collect()
    };
    filled.concat()
}
