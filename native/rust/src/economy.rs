//! The economy (spec 39 step 9b-ii), ported from GDScript's EconomySystem and
//! CaptureResolution as they were (specs/03-resource-nodes-and-workers.md).
//! - Five pools (food, wood, stone, metal, crystal), each starting at 0. Any other name reads
//!   0 and is made by adding to it, as GDScript's dictionary was. A spend that can't be
//!   afforded spends nothing.
//! - A harvester's yield falls 10% each whole interval since its capture, rounded half away
//!   from zero (GDScript's `round`), and never below its floor. The power is `det_pow`'s, the
//!   one GDScript's DetMath gave.
//! - A captured node is held by `Captures` (`economy/captures.rs`): a resource node harvests
//!   until it is ravaged, and a fort awaits a choice (fortify or dismantle). Pure.

#[cfg(test)]
mod tests;

mod binding;
mod captures;

use std::collections::BTreeMap;

pub(crate) use binding::{node_of, EconomyState};
pub use captures::{Captures, Choice};

/// The pools' names, in the resource bar's order.
const RESOURCES: [&str; 5] = ["food", "wood", "stone", "metal", "crystal"];

/// A yield's fall each whole interval (the parent spec's "-10% every N rounds").
const DECAY_RATE_PER_INTERVAL: f64 = 0.9;

/// The resource pools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Economy {
    pools: BTreeMap<String, i64>,
}

impl Default for Economy {
    fn default() -> Self {
        Self { pools: RESOURCES.iter().map(|name| ((*name).to_owned(), 0)).collect() }
    }
}

impl Economy {
    pub fn balance(&self, resource: &str) -> i64 {
        self.pools.get(resource).copied().unwrap_or(0)
    }

    pub fn add(&mut self, resource: &str, amount: i64) {
        *self.pools.entry(resource.to_owned()).or_insert(0) += amount;
    }

    /// Spends `amount` of `resource` when there is that much; whether it did.
    pub fn spend(&mut self, resource: &str, amount: i64) -> bool {
        if self.balance(resource) < amount {
            return false;
        }
        self.add(resource, -amount);
        true
    }
}

/// `base_yield` after `ticks_elapsed` ticks of whole `interval`s, at least `floor`. An
/// interval of 0 or less counts no intervals (GDScript's division by zero was an error).
pub fn decayed_yield(base_yield: i64, ticks_elapsed: i64, interval: i64, floor: i64) -> i64 {
    let intervals = if interval > 0 { ticks_elapsed / interval } else { 0 };
    let decayed =
        base_yield as f64 * crate::det_pow::pow(DECAY_RATE_PER_INTERVAL, intervals as f64);
    floor.max(decayed.round() as i64)
}
