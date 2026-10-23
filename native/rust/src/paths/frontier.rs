//! The open cells of a path search (PathFrontier, spec 30), the most promising first: a
//! binary heap ordered by the estimated time of a way through the cell, then by how far the
//! cell lies aside from the straight way, then by how near it is to the goal, then by its
//! place on the grid (row, then column). Geometry decides every tie, never the order the
//! cells were found in (Decision 97). Each measure is rounded to a thousandth, so float
//! noise in sums taken in different orders can't pre-empt the geometry.

/// A thousandth: what each measure is rounded to.
const GRAIN: f64 = 1000.0;
/// Room for the second measure (aside, in thousandths) under the first, and for the cell's
/// index under the third.
const ASIDE_ROOM: i64 = 1 << 24;
const CELL_ROOM: i64 = 1 << 24;

/// Two keys an entry: the estimate and its distance aside; its nearness to the goal and its
/// cell's index (which the entry gives back).
#[derive(Default)]
pub struct Frontier {
    first: Vec<i64>,
    second: Vec<i64>,
}

impl Frontier {
    pub fn is_empty(&self) -> bool {
        self.first.is_empty()
    }

    /// Adds `cell` (an index on the grid) with its estimated time `estimate` of a way
    /// through it, its distance `aside` from the straight way and `remaining` to the goal.
    pub fn push(&mut self, cell: usize, estimate: f64, aside: f64, remaining: f64) {
        let first =
            roundi(estimate * GRAIN) * ASIDE_ROOM + roundi(aside * GRAIN).min(ASIDE_ROOM - 1);
        let second = roundi(remaining * GRAIN) * CELL_ROOM + cell as i64;
        let mut at = self.first.len();
        self.first.push(first);
        self.second.push(second);
        while at > 0 {
            // sift the hole up past every entry after it
            let up = (at - 1) >> 1;
            if self.first[up] < first || (self.first[up] == first && self.second[up] < second) {
                break;
            }
            self.first[at] = self.first[up];
            self.second[at] = self.second[up];
            at = up;
        }
        self.first[at] = first;
        self.second[at] = second;
    }

    /// Takes the most promising cell off the heap.
    pub fn pop(&mut self) -> usize {
        let top = (self.second[0] % CELL_ROOM) as usize;
        let last = self.first.len() - 1;
        let first = self.first[last];
        let second = self.second[last];
        self.first.truncate(last);
        self.second.truncate(last);
        if last == 0 {
            return top;
        }
        let mut at = 0;
        loop {
            // sift the hole down past every entry before the last
            let mut child = 2 * at + 1;
            if child >= last {
                break;
            }
            if child + 1 < last && self.before(child + 1, child) {
                child += 1;
            }
            if first < self.first[child]
                || (first == self.first[child] && second < self.second[child])
            {
                break;
            }
            self.first[at] = self.first[child];
            self.second[at] = self.second[child];
            at = child;
        }
        self.first[at] = first;
        self.second[at] = second;
        top
    }

    fn before(&self, one: usize, other: usize) -> bool {
        if self.first[one] != self.first[other] {
            return self.first[one] < self.first[other];
        }
        self.second[one] < self.second[other]
    }
}

/// Godot's `roundi`: half away from zero.
fn roundi(x: f64) -> i64 {
    x.round() as i64
}
