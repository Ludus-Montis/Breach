//! tools/native_bench.gd in the crate (spec 39 step 8c-xix), for `breach_native_bench`: a
//! mid-fight tick of FormationBench's clash on one thread ("rust") and on the core's pool
//! ("rust_threads"), and the two passes native_bench.gd times inside it - the bodies' parting
//! and the scrum's seek (its sync and search: the `scrum: seek` lap; the seek, walk and
//! facing until spec 41 step 1 split them). The battle is brought to its fight as
//! native_bench.gd brings it (`phases::settled`: to its first engagement, then `settle` ticks),
//! then `fight` ticks are timed. native_bench.gd's b.sync, b.call and b.write columns time
//! the Godot boundary, which the crate's tick doesn't cross, and its b.core is the crate's
//! whole parting: the crate prints tick, rest, bodies and seek.

#[cfg(test)]
mod tests;

use std::time::Instant;

use crate::content::Content;
use crate::phases::{self, Settled};
use crate::threads::Threads;

/// The columns, ms a tick each. "rest" is the tick less the bodies and the seek.
pub const COLUMNS: [&str; 4] = ["tick", "rest", "bodies", "seek"];

/// native_bench.gd's arguments for one run, with its defaults.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub settle: usize,
    pub fight: usize,
    pub seed: i64,
}

impl Default for Options {
    fn default() -> Options {
        Options { settle: 10, fight: 10, seed: 1 }
    }
}

/// The clash of `per_side` a side, `width` wide, brought to its fight (native_bench.gd's
/// `_measure` before its clock starts); None if it can't be set up.
pub fn settled(
    per_side: usize,
    width: usize,
    threaded: Threads,
    options: &Options,
    content: &Content,
) -> Option<Settled> {
    let given = phases::Options {
        side: per_side,
        width,
        settle: options.settle,
        fight: options.fight,
        seed: options.seed,
        threaded,
        ..phases::Options::default()
    };
    phases::settled(&given, content)
}

/// `_measure`: the average ms a tick of each column over `fight` ticks of `settled`.
pub fn measure(settled: &mut Settled, fight: usize, content: &Content) -> [f64; 4] {
    let (mut bodies, mut seek) = (0_i64, 0_i64);
    let began = Instant::now();
    for _ in 0..fight {
        if let Some(ticked) = settled.ticked(content) {
            bodies += ticked.ended.parting_usec;
            seek += ticked.seek_usec;
        }
    }
    let ticks = fight.max(1) as f64 * 1000.0;
    let tick = began.elapsed().as_secs_f64() * 1e6 / ticks;
    let (bodies, seek) = (bodies as f64 / ticks, seek as f64 / ticks);
    [tick, tick - bodies - seek, bodies, seek]
}

/// `_pick`: each column's value at quantile `at` (0: min, 0.5: median) across the runs.
pub fn pick(results: &[[f64; 4]], at: f64) -> [f64; 4] {
    let mut out = [0.0; 4];
    for (column, value) in out.iter_mut().enumerate() {
        let mut values: Vec<f64> = results.iter().map(|r| r[column]).collect();
        values.sort_by(f64::total_cmp);
        if !values.is_empty() {
            *value = values[(at * (values.len() - 1) as f64 + 0.5) as usize];
        }
    }
    out
}

/// The heading line.
pub fn heading() -> String {
    let cells: Vec<String> = COLUMNS.iter().map(|c| format!("{c:>7}")).collect();
    format!("{:<17} {:>5} {:>3}  {}", "engine", "side", "w", cells.join(" "))
}

/// One run's or one pick's line.
pub fn line(label: &str, (side, width): (usize, usize), values: &[f64; 4]) -> String {
    let cells: Vec<String> = values.iter().map(|v| format!("{v:>7.2}")).collect();
    format!("{label:<17} {side:>5} {width:>3}  {}", cells.join(" "))
}
