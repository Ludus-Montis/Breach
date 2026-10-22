//! Serialising the field's hash maps and its heap in a fixed order (spec 39 step 1), so the
//! same state always writes the same bytes. Read back, each is its own type again.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

use serde::{Serialize, Serializer};

/// A map, written in key order.
pub fn map<S: Serializer, K: Ord + Serialize, V: Serialize>(
    map: &HashMap<K, V>,
    out: S,
) -> Result<S::Ok, S::Error> {
    map.iter().collect::<BTreeMap<_, _>>().serialize(out)
}

/// A min-heap of slots, written lowest first.
pub fn heap<S: Serializer>(heap: &BinaryHeap<Reverse<u32>>, out: S) -> Result<S::Ok, S::Error> {
    let mut slots: Vec<&Reverse<u32>> = heap.iter().collect();
    slots.sort_by(|a, b| b.cmp(a));
    slots.serialize(out)
}
