//! The furthest reach over each unit's line (spec 39 step 10b-iii): what `Shape::edge_units`
//! compares each unit with, worked out in n log n rather than by every pair.
//!
//! Two lines overlap when `overlap(a, b) > EPSILON`, that is `min(a.1, b.1) - max(a.0, b.0)`
//! (in f64). A rounded difference only grows with its first term and shrinks with its second,
//! so that one difference is the least of `a.1 - b.0`, `b.1 - a.0`, `a.1 - a.0` and
//! `b.1 - b.0`, and the lines overlap exactly when all four exceed the tolerance. Each of the
//! first two picks out a run of the lines sorted by one end, so the lines a unit overlaps are
//! those in a prefix by start that also lie in a prefix by end, read with a tree of maxima.

use super::{Extent, EPSILON};

/// The rounded `high - low` that `overlap` compares.
fn apart(high: f32, low: f32) -> f64 {
    high as f64 - low as f64
}

/// For each line, the furthest reach of the lines (itself among them) it overlaps by more than
/// the tolerance, or negative infinity for none.
pub fn furthest(spans: &[Extent], reaches: &[f32]) -> Vec<f32> {
    let wide = |s: Extent| apart(s.1, s.0) > EPSILON;
    let mut by_start: Vec<usize> = (0..spans.len()).filter(|&o| wide(spans[o])).collect();
    by_start.sort_by(|&a, &b| spans[a].0.total_cmp(&spans[b].0));
    let mut by_end = by_start.clone();
    by_end.sort_by(|&a, &b| spans[b].1.total_cmp(&spans[a].1)); // furthest end first
    let mut end_rank = vec![0; spans.len()];
    for (rank, &o) in by_end.iter().enumerate() {
        end_rank[o] = rank;
    }
    // Each wide line's query: how many lines by start begin early enough, and by end end late enough.
    let mut queries: Vec<(usize, usize, usize)> = (0..spans.len())
        .filter(|&u| wide(spans[u]))
        .map(|u| {
            let (low, high) = spans[u];
            let starts = by_start.partition_point(|&o| apart(high, spans[o].0) > EPSILON);
            let ends = by_end.partition_point(|&o| apart(spans[o].1, low) > EPSILON);
            (starts, ends, u)
        })
        .collect();
    queries.sort_unstable();
    let mut out = vec![f32::NEG_INFINITY; spans.len()];
    let mut tree = Maxima::new(by_end.len());
    let mut added = 0;
    for (starts, ends, u) in queries {
        for &o in &by_start[added..starts] {
            tree.raise(end_rank[o], reaches[o]);
        }
        added = added.max(starts);
        out[u] = tree.most(ends);
    }
    out
}

/// A Fenwick tree of maxima: raise a place, read the most of the first places.
struct Maxima(Vec<f32>);

impl Maxima {
    fn new(len: usize) -> Maxima {
        Maxima(vec![f32::NEG_INFINITY; len + 1])
    }

    fn raise(&mut self, place: usize, value: f32) {
        let mut i = place + 1;
        while i < self.0.len() {
            self.0[i] = self.0[i].max(value);
            i += i.isolate_lowest_one();
        }
    }

    /// The most over the first `count` places.
    fn most(&self, count: usize) -> f32 {
        let (mut i, mut most) = (count, f32::NEG_INFINITY);
        while i > 0 {
            most = most.max(self.0[i]);
            i -= i.isolate_lowest_one();
        }
        most
    }
}
