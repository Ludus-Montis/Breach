//! Each body's sum of its pushes in a pass, on one thread: each pair's pushes added to its
//! bodies' sums in pair order, noting each body when it is first pushed.

use super::{framed, moves_along, sums, Bodies, Push};
use crate::maths::V2;

/// The one-thread sums: each pair's pushes added to its bodies' sums in pair order, the
/// buffers kept across passes.
pub(super) struct Serial {
    moves: Vec<V2>,
    has_move: Vec<bool>,
    // pushed by a body out of its frame: one in its frame leaves it (UnitBodies._move)
    loosens: Vec<bool>,
    order: Vec<usize>,
}

impl Serial {
    pub(super) fn new(count: usize) -> Serial {
        Serial {
            moves: vec![V2::ZERO; count],
            has_move: vec![false; count],
            loosens: vec![false; count],
            order: Vec::with_capacity(count),
        }
    }

    pub(super) fn summed(
        &mut self,
        bodies: &Bodies,
        pairs: &[(usize, usize)],
        pushes: impl Iterator<Item = Push>,
        way: &mut dyn FnMut(usize, usize) -> V2,
    ) -> Vec<sums::Moved> {
        self.order.clear();
        for (&(i, j), p) in pairs.iter().zip(pushes) {
            let loosening = !(framed(bodies.flags[i]) && framed(bodies.flags[j]));
            let p = match p {
                Push::Coincident { depth, share } => moves_along(way(i, j), depth, share),
                p => p,
            };
            let Push::Moves(one, two) = p else { continue };
            if let Some(v) = one {
                accumulate(&mut self.moves, &mut self.has_move, &mut self.order, i, v, true);
                self.loosens[i] |= loosening;
            }
            if let Some(v) = two {
                accumulate(&mut self.moves, &mut self.has_move, &mut self.order, j, v, false);
                self.loosens[j] |= loosening;
            }
        }
        let moved = self.order.iter().map(|&k| (k, self.moves[k], self.loosens[k])).collect();
        for &k in &self.order {
            self.moves[k] = V2::ZERO;
            self.has_move[k] = false;
            self.loosens[k] = false;
        }
        moved
    }
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
