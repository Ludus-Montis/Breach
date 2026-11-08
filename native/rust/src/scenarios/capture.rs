//! What a capture writes beside its movie, `capture.json` (spec 44 step 2c): the scenario's
//! name, map, seed and record, the commit, the ticks of its four moments (the first tick, first
//! contact, the midpoint between them and the last, and the last tick), the ticks played, each
//! behaviour with the first tick it was seen or null, and the behaviours it is named for and
//! didn't show. Step 3's stills are cut at the moments.

use serde_json::{json, Map, Value};

use super::{Scenario, Seen, BEHAVIOURS};

/// `scenario`'s capture at commit `commit`, having played `ticks` ticks and seen `seen`. With no
/// contact, the contact moment is null and the midpoint is the middle of the whole play.
pub fn capture(scenario: &Scenario, seen: &Seen, ticks: i64, commit: &str) -> Value {
    let last = (ticks - 1).max(0);
    let midpoint = (seen.contact.unwrap_or(0) + last) / 2;
    let behaviours: Map<String, Value> =
        BEHAVIOURS.iter().map(|&b| (b.to_owned(), json!(seen.at(b)))).collect();
    json!({
        "scenario": scenario.name,
        "map": scenario.map,
        "seed": scenario.record.seed,
        "record": scenario.text,
        "commit": commit,
        "moments": {"first": 0, "contact": seen.contact, "midpoint": midpoint, "last": last},
        "ticks": ticks,
        "behaviours": behaviours,
        "missing": seen.missing(),
    })
}
