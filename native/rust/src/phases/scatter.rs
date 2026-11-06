//! tick_phases' `scatter=1` (spec 39 step 10a, Decision 159): how scattered each squad's units
//! are across the unit store's slots, so slot packing (step 10e) is measured before it is
//! built. A squad is packed when its members' slots are one run of consecutive slots. Two
//! measures over every squad with members: its runs (1 when packed) and the slots it spans,
//! from its lowest to its highest (its members' count when packed). It reads the squads and
//! writes nothing.

#[cfg(test)]
mod tests;

use crate::squads::Squads;

/// The squads' scatter, summed over the squads with members.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scatter {
    pub squads: usize,
    pub members: usize,
    /// Runs of consecutive slots: `squads` when every squad is packed.
    pub runs: usize,
    /// Slots from each squad's lowest to its highest: `members` when every squad is packed.
    pub span: usize,
}

/// Every squad's scatter over its members' slots, summed.
pub fn measured(squads: &Squads) -> Scatter {
    let mut scatter = Scatter::default();
    for members in squads.members.iter().filter(|members| !members.is_empty()) {
        let mut slots = members.clone();
        slots.sort_unstable();
        let breaks = slots.windows(2).filter(|pair| pair[1] - pair[0] > 1).count();
        scatter.squads += 1;
        scatter.members += slots.len();
        scatter.runs += breaks + 1;
        scatter.span += (slots[slots.len() - 1] - slots[0]) as usize + 1;
    }
    scatter
}

/// The line `scatter=1` prints: the runs and the span, each beside what packed would be.
pub fn line(when: &str, scatter: &Scatter) -> String {
    let Scatter { squads, members, runs, span } = scatter;
    format!(
        "scatter {when}: {squads} squads, {members} members in {runs} runs ({squads} packed), \
         spanning {span} slots ({members} packed)"
    )
}
