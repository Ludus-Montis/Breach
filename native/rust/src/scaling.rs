//! tools/scaling_check.gd in the crate (spec 39 step 8c-xvii), for `breach_scaling`: a tick's
//! cost must grow no faster than its units, n log n at worst (bounded work). The clash at each
//! size, doubling, `ranks` deep, marching and mid-fight, then `stacks` lanes of `stack` a side,
//! `settle` ticks into their fights; each size's tick is the least of `runs` timed spans of
//! `ticks` ticks. A doubling that costs more than `limit` times as much fails. Printed as
//! scaling_check.gd prints it, so the untimed columns can be compared line for line; the
//! timings are the crate's own, so only its verdict on them counts.

#[cfg(test)]
mod tests;

use std::time::Instant;

use crate::battles::Battle;
use crate::content::Content;
use crate::events::{Log, ENGAGED};
use crate::field::Field;
use crate::play::{starts, told, Play};
use crate::squads::Squads;

/// scaling_check.gd's STAGES, in the order it runs them.
pub const STAGES: [&str; 3] = ["march", "fight", "stacks"];
/// FormationBench.CONTACT_LIMIT: ticks a battle may take to come to contact.
pub const CONTACT_LIMIT: i64 = 2000;
/// The first line scaling_check.gd prints.
pub const HEADING: &str = "scaling_check: ms a tick, and its growth per doubling";
/// The line it prints last when a doubling was too steep.
pub const FAILED: &str = "FAILED: run tools/tick_phases.gd at the sizes above to find the pass";

/// scaling_check.gd's arguments, with its defaults, and whether the tick is threaded.
#[derive(Clone, Debug)]
pub struct Options {
    pub sizes: Vec<usize>,
    pub ranks: usize,
    pub ticks: usize,
    pub runs: usize,
    pub limit: f64,
    pub stacks: Vec<usize>,
    pub stack: usize,
    pub settle: usize,
    pub threaded: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            sizes: vec![128, 256, 512],
            ranks: 8,
            ticks: 6,
            runs: 3,
            limit: 2.5,
            stacks: vec![1, 2, 4],
            stack: 128,
            settle: 20,
            threaded: true,
        }
    }
}

/// A battle ready to be timed, on its own crate.
pub struct Ready {
    pub play: Play,
    pub squads: Squads,
    pub field: Field,
    log: Log,
}

impl Ready {
    /// One tick; how many events it told of.
    pub fn step(&mut self, content: &Content) -> usize {
        self.play.step(&mut self.squads, &mut self.field, content, &mut self.log);
        starts(&self.log.take()).count()
    }

    /// One tick; whether it told of an engagement.
    fn engages(&mut self, content: &Content) -> bool {
        self.play.step(&mut self.squads, &mut self.field, content, &mut self.log);
        told(&self.log.take(), ENGAGED)
    }
}

/// scaling_check.gd's `_ready_for`: the clash at `size` a side (FormationBench.clash, `ranks`
/// deep), three ticks into its march or `settle` 5 ticks past its first engagement; for
/// "stacks", `size` lanes of `stack` a side (FormationBench.lanes), `settle` ticks past it.
/// Blows aren't rolled, as FormationSimulation's aren't by default. None if it can't be set up.
pub fn ready(
    stage: &str,
    size: usize,
    battle_seed: i64,
    options: &Options,
    content: &Content,
) -> Option<Ready> {
    let ranks = options.ranks.max(1);
    let (battle, settle) = if stage == "stacks" {
        let width = (options.stack / ranks).max(1);
        (Battle::Lanes { lanes: size, per_squad: options.stack, width, archers: 0 }, options.settle)
    } else {
        (Battle::Clash { per_side: size, width: (size / ranks).max(1), archers: 0 }, 5)
    };
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let at = (battle_seed, false, options.threaded);
    let play = Play::new(&battle, (&mut squads, &mut field), content, at)?;
    let mut ready = Ready { play, squads, field, log: Log::default() };
    if stage == "march" {
        for _ in 0..3 {
            ready.step(content);
        }
        return Some(ready);
    }
    let mut contact = false;
    while !contact && ready.play.tick < CONTACT_LIMIT {
        contact = ready.engages(content);
    }
    for _ in 0..settle {
        ready.step(content);
    }
    Some(ready)
}

/// scaling_check.gd's `_least`: the least ms a tick over `runs` spans of `ticks` ticks of
/// `stage` at `size`, each run on a fresh battle seeded by its number from 1.
pub fn least(stage: &str, size: usize, options: &Options, content: &Content) -> Option<f64> {
    let mut least = f64::INFINITY;
    for run in 0..options.runs {
        let mut ready = ready(stage, size, run as i64 + 1, options, content)?;
        let began = Instant::now();
        for _ in 0..options.ticks {
            ready.step(content);
        }
        let ms = began.elapsed().as_secs_f64() * 1000.0 / options.ticks as f64;
        least = least.min(ms);
    }
    Some(least)
}

/// The sizes `stage` runs at.
pub fn sizes<'a>(stage: &str, options: &'a Options) -> &'a [usize] {
    if stage == "stacks" {
        &options.stacks
    } else {
        &options.sizes
    }
}

/// `stage`'s lines, one for each size and its ms a tick in `spent`, as scaling_check.gd
/// prints them, and how many doublings grew by more than `limit`.
pub fn lines(stage: &str, sizes: &[usize], spent: &[f64], limit: f64) -> (Vec<String>, usize) {
    let (mut out, mut failures, mut last) = (Vec::new(), 0, 0.0);
    let what = if stage == "stacks" { "stacks" } else { "a side" };
    for (&size, &ms) in sizes.iter().zip(spent) {
        let growth = if last > 0.0 { ms / last } else { 0.0 };
        let verdict =
            if growth <= limit { String::new() } else { format!("  TOO STEEP (limit {limit:.1})") };
        failures += usize::from(!verdict.is_empty());
        let shown = if last == 0.0 { String::new() } else { format!("x{growth:.2}") };
        out.push(format!("  {stage:<6} {size:>5} {what} {ms:>9.2} ms  {shown}{verdict}"));
        last = ms;
    }
    (out, failures)
}
