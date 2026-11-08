//! tools/tick_phases.gd in the crate (spec 39 step 8c-xviii), for `breach_tick_phases`: where
//! a mid-fight tick's time goes, pass by pass. FormationBench's clash (or its lanes) is set up
//! and brought to its fight as tick_phases.gd brings it (`settled`), then `fight` ticks are
//! timed with the tick's laps counted (`laps`), each phase's ms a tick printed largest first
//! as tick_phases.gd prints them, so `bench_compare.py` reads both. The phases are the crate's
//! own passes, named as tick_phases.gd names the GDScript passes they ported where they match.
//! A second battle steps untimed beside it, and the two must end equal, or it says so.

pub mod counts;
pub mod presets;
pub mod scatter;
pub mod series;
#[cfg(test)]
pub(crate) mod tests;

use crate::battles::Battle;
use crate::content::Content;
use crate::digest::state::state;
use crate::events::{Log, ENGAGED};
use crate::field::Field;
use crate::laps;
use crate::locks::RETREAT;
use crate::orders;
use crate::play::{starts, told, Play};
use crate::squads::Squads;
use crate::threads::Threads;
use crate::tick::Ticked;

/// FormationBench.CONTACT_LIMIT: ticks a battle may take to come to contact.
pub const CONTACT_LIMIT: i64 = 2000;
/// The line printed when the timed battle and the untimed one end apart.
pub const WARNING: &str = "WARNING: breach_tick_phases' laps changed the battle - fix them";

/// tick_phases.gd's arguments, with its defaults, and whether the tick is threaded.
#[derive(Clone, Debug)]
pub struct Options {
    pub side: usize,
    pub width: usize,
    pub settle: usize,
    pub fight: usize,
    pub seed: i64,
    /// "march" times the march before contact, `settle` ticks after the start.
    pub march: bool,
    /// The squad ordered to retreat once the fight has settled in.
    pub retreat: Option<i64>,
    pub squads: usize,
    pub archers: usize,
    pub threaded: Threads,
    /// `terrain=mixed`: the clash on FormationBench's mixed ground (one lane only).
    pub mixed: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            side: 1024,
            width: 32,
            settle: 10,
            fight: 5,
            seed: 1,
            march: false,
            retreat: None,
            squads: 1,
            archers: 0,
            threaded: Threads::Pool,
            mixed: false,
        }
    }
}

/// A battle brought to where it is timed, on its own crate.
pub struct Settled {
    pub play: Play,
    pub squads: Squads,
    pub field: Field,
    log: Log,
}

impl Settled {
    /// One tick; how many events it told of.
    pub fn step(&mut self, content: &Content) -> usize {
        self.play.step(&mut self.squads, &mut self.field, content, &mut self.log);
        starts(&self.log.take()).count()
    }

    /// One tick, and what it did; its events dropped.
    pub(crate) fn ticked(&mut self, content: &Content) -> Option<Ticked> {
        let ticked = self.play.step(&mut self.squads, &mut self.field, content, &mut self.log);
        self.log.take();
        ticked
    }

    /// One tick; whether it told of an engagement.
    fn engages(&mut self, content: &Content) -> bool {
        self.play.step(&mut self.squads, &mut self.field, content, &mut self.log);
        told(&self.log.take(), ENGAGED)
    }

    /// The listed squads' state as the digest records it, as bytes.
    pub fn state(&self) -> Vec<u8> {
        state(&self.squads, &self.field, &self.play.listed).bytes()
    }
}

/// tick_phases.gd's `_settled`: FormationBench.lanes (`squads` lanes of `side / squads` a
/// side), its clash or (`mixed`) FormationBench.mixed, `archers` the back of each grem
/// squad; for the march, `settle` ticks
/// from the start; for the fight, played till it tells of an engagement (or CONTACT_LIMIT),
/// then `settle` ticks more, and `retreat`'s order given. None if it can't be set up.
pub fn settled(options: &Options, content: &Content) -> Option<Settled> {
    let (side, width, archers) = (options.side, options.width, options.archers);
    let battle = if options.mixed {
        Battle::Mixed { per_side: side, width, archers }
    } else if options.squads > 1 {
        let lanes = options.squads;
        Battle::Lanes { lanes, per_squad: side / lanes, width, archers }
    } else {
        Battle::Clash { per_side: side, width, archers }
    };
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let at = (options.seed, false, options.threaded);
    let play = Play::new(&battle, (&mut squads, &mut field), content, at)?;
    let mut settled = Settled { play, squads, field, log: Log::default() };
    if !options.march {
        let mut contact = false;
        while !contact && settled.play.tick < CONTACT_LIMIT {
            contact = settled.engages(content);
        }
    }
    for _ in 0..options.settle {
        settled.step(content);
    }
    if let (false, Some(id)) = (options.march, options.retreat) {
        orders::queue(&mut settled.squads, id, RETREAT);
    }
    Some(settled)
}

/// `ticks` ticks of `settled`, timed: each phase's microseconds summed over them, in the
/// order first lapped.
pub fn timed(settled: &mut Settled, ticks: usize, content: &Content) -> Vec<(&'static str, f64)> {
    let mut spent: Vec<(&'static str, f64)> = Vec::new();
    for _ in 0..ticks {
        let (_, laps) = laps::timed(|| settled.step(content));
        for (phase, usec) in laps {
            match spent.iter_mut().find(|(name, _)| *name == phase) {
                Some((_, total)) => *total += usec,
                None => spent.push((phase, usec)),
            }
        }
    }
    spent
}

/// The lines tick_phases.gd prints after its machine line: the battle and its ms a tick, then
/// each phase's ms a tick, largest first, over `ticks` ticks (`fight`, or a series' length).
pub fn lines(options: &Options, ticks: usize, spent: &[(&'static str, f64)]) -> Vec<String> {
    let battle = format!("{} a side, {} wide", options.side, options.width);
    headed((&battle, 1), ticks, spent)
}

/// `lines` for any `battle`: "<battle>: <ms> ms a tick", to `decimals` places, then each
/// phase's ms a tick, largest first, over `ticks` ticks.
pub fn headed(
    (battle, decimals): (&str, usize),
    ticks: usize,
    spent: &[(&'static str, f64)],
) -> Vec<String> {
    let ticks = ticks.max(1) as f64;
    let total: f64 = spent.iter().map(|(_, usec)| usec).sum();
    let mut out = vec![format!("{battle}: {:.decimals$} ms a tick", total / ticks / 1000.0)];
    let mut phases = spent.to_vec();
    phases.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (phase, usec) in phases {
        out.push(format!("  {:<32} {:>8.2}", phase, usec / ticks / 1000.0));
    }
    out
}

/// tick_phases.gd's machine line: the processor, its threads and the system, as Godot names
/// them.
pub fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            let line = info.lines().find(|line| line.starts_with("model name"))?.to_string();
            Some(line.split_once(':')?.1.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let system = match std::env::consts::OS {
        "linux" => "Linux",
        "windows" => "Windows",
        "macos" => "macOS",
        other => other,
    };
    format!("machine: {cpu}, {threads} threads, {system}")
}
