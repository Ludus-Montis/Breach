//! Scratch space the tick's passes keep on the field between ticks (spec 41 step 4): each
//! pass clears its buffers and fills them afresh, so their old contents never reach a
//! result. Not state: a snapshot doesn't write them, and a field read back starts them
//! empty, to the same results.

use crate::parting;
use crate::scrum;

/// Every pass's kept buffers.
#[derive(Default)]
pub struct Buffers {
    /// The seek's crowd, claims and seekers (`scrum::plan`).
    pub seek: scrum::Buffers,
    /// Parting's positions, pairs, pushes, sums and neighbour list (`parting::part_in`).
    pub parting: parting::Buffers,
}

impl Buffers {
    /// True once a pass has kept something in them.
    pub fn warm(&self) -> bool {
        self.seek.warm() || self.parting.warm()
    }
}
