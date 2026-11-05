//! tick_phases' `preset=` (spec 39 step 10a, Decision 159): the two battles the bench is named
//! by, so a run asks for the target or the daily bench by name rather than by its numbers.
//! Both are a series from contact of lanes of 1,024 a side: `4096` is four of them (the daily
//! bench), `25k` is 24 (the target, 24,576 a side). Lanes the same size let the scaling check
//! (`ci/godot/scripts/bench_scaling.py`) expect the tick to grow by the ratio of the squads.
//! Each series runs to `LIMIT` ticks at most. A series ends when either side has no squad
//! that isn't routing: at 4096 that is 16 ticks after contact (tick 91 to 107), so the limit
//! covers the fight with room to spare.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use super::series::LIMIT;

/// Each preset by name: the arguments it stands for, as `side`, `squads`, `series`, `limit`.
const PRESETS: [(&str, [usize; 4]); 2] =
    [("4096", [4096, 4, 1, LIMIT]), ("25k", [24576, 24, 1, LIMIT])];
const KEYS: [&str; 4] = ["side", "squads", "series", "limit"];

/// Fills in `preset`'s arguments where none was given beside it (one given wins); an unknown
/// preset is refused, naming those there are. With no `preset`, `args` is left as it is.
pub fn expand(args: &mut HashMap<String, String>) -> Result<(), String> {
    let Some(name) = args.get("preset").cloned() else { return Ok(()) };
    let Some((_, values)) = PRESETS.iter().find(|(preset, _)| *preset == name) else {
        let known: Vec<&str> = PRESETS.iter().map(|(preset, _)| *preset).collect();
        return Err(format!("unknown preset {name} (there are {})", known.join(" and ")));
    };
    for (key, value) in KEYS.iter().zip(values) {
        args.entry(key.to_string()).or_insert_with(|| value.to_string());
    }
    Ok(())
}
