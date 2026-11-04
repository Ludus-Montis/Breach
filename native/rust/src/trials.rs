//! BattleTrials played in the crate (spec 39 step 8c-xiv), for the cargo binary
//! `breach_trials` (src/bin/trials.rs): each scenario fought once per battle seed with nothing
//! from Godot (Decision 93), and its wins and losses summed and printed as
//! tools/formation_trials.gd prints them, so the two outputs can be compared line for line
//! (CI's `trials` job does). The mirrors are `battles::Battle::Mirror` played by `play::Play`
//! till a side no longer stands; the field's scenarios are `field_play::FieldPlay`, its waves
//! built (by BattleTrials' eight grem builders), then sent as the scenario sends them, played till the kingdom's line breaks.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use crate::battles::Battle;
use crate::content::Content;
use crate::events::{Log, DIED, DOWNED, SURRENDERED};
use crate::field::Field;
use crate::field_play::{FieldPlay, A, B};
use crate::locks;
use crate::play::{starts, Play};
use crate::squads::Squads;

/// BattleTrials.SCENARIOS, in its order.
pub const SCENARIOS: [&str; 6] =
    ["mirror_headon", "mirror_flank", "field_a", "field_b", "field_b_waits", "field_together"];
/// BattleTrials.LIMIT_TICKS: the most ticks a run (and a field's building) plays.
pub const LIMIT_TICKS: i64 = 2000;
/// BattleTrials.FACTIONS: the sides whose losses are counted and who can win.
pub const FACTIONS: [&str; 2] = ["player", "the_kingdom"];
/// The waves' units built before a field's scenario sends them: A's eight grems, and B's
/// eight with its chieftain.
const BUILT: [i64; 2] = [8, 9];
/// BattleTrials' field's grem builders (the digest's field has FormationFieldActions' four).
const GREM_BUILDERS: usize = 8;
/// How long field_b_waits plays after sending B before it sends A.
const WAITS_TICKS: i64 = 90;

/// The trials' options (formation_trials.gd's): blows rolled, the kingdom's line led by a
/// captain, the first battle seed, a mirror's sides spawned the other way round; and whether
/// the tick runs threaded (outcomes don't change).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub rolls: bool,
    pub captain: bool,
    pub first_seed: i64,
    pub swap: bool,
    pub threaded: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { rolls: true, captain: false, first_seed: 1, swap: false, threaded: true }
    }
}

/// A run's result (BattleTrials'): the units each side lost (FACTIONS' order), the side that
/// won (an index into FACTIONS, or None) and the ticks it took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trial {
    pub lost: [i64; 2],
    pub winner: Option<usize>,
    pub ticks: i64,
}

/// BattleTrials.run: `scenario` fought `runs` times, seeded from `options.first_seed` on;
/// None for a scenario that isn't one of SCENARIOS, or whose battle can't be set up from
/// `content`.
pub fn run(scenario: &str, runs: i64, options: &Options, content: &Content) -> Option<Vec<Trial>> {
    if !SCENARIOS.contains(&scenario) {
        return None;
    }
    (0..runs.max(0))
        .map(|index| {
            let seed = options.first_seed + index;
            if scenario.starts_with("mirror") {
                mirror(scenario == "mirror_flank", seed, options, content)
            } else {
                field(scenario, seed, options, content)
            }
        })
        .collect()
}

/// A mirror fought on a fresh crate till fewer than both sides stand, or LIMIT_TICKS.
fn mirror(flank: bool, seed: i64, options: &Options, content: &Content) -> Option<Trial> {
    let (mut squads, mut field, mut log) = (Squads::default(), Field::default(), Log::default());
    let battle = Battle::Mirror { flank, swap: options.swap };
    let at = (seed, options.rolls, options.threaded);
    let mut play = Play::new(&battle, (&mut squads, &mut field), content, at)?;
    let mut lost = Lost::default();
    for tick in 0..LIMIT_TICKS {
        play.step(&mut squads, &mut field, content, &mut log);
        lost.count(&log.take(), &field);
        let standing: Vec<usize> =
            (0..FACTIONS.len()).filter(|&f| play.stands(&squads, &field, FACTIONS[f])).collect();
        if standing.len() < 2 {
            let winner = standing.first().copied();
            return Some(Trial { lost: lost.by_side, winner, ticks: tick + 1 });
        }
    }
    Some(Trial { lost: lost.by_side, winner: None, ticks: LIMIT_TICKS })
}

/// A field scenario: the field (its line captained if asked) on a fresh crate, played till
/// both waves are built, then sent as the scenario sends them and played till the kingdom's
/// line is destroyed or routs (the player wins) or LIMIT_TICKS (the kingdom does).
fn field(scenario: &str, seed: i64, options: &Options, content: &Content) -> Option<Trial> {
    let (mut squads, mut field_, mut log) = (Squads::default(), Field::default(), Log::default());
    let at = (seed, options.rolls, options.threaded);
    let mut play = FieldPlay::new(options.captain, (&mut squads, &mut field_), content, at)?;
    let grem = content.units.get("grem")?;
    play.builders.set("grem", grem.build_seconds, GREM_BUILDERS); // all idle, as if made so
    let line = *play.kingdom.first()?;
    for _ in 0..LIMIT_TICKS {
        if play.waves[A].built() == BUILT[0] && play.waves[B].built() == BUILT[1] {
            break;
        }
        play.step(&mut squads, &mut field_, content, &mut log);
        log.take();
    }
    let mut lost = Lost::default();
    match scenario {
        "field_a" => {
            play.send(A, 0, (&mut squads, &mut field_), content);
        }
        "field_b" => {
            play.send(B, 0, (&mut squads, &mut field_), content);
        }
        "field_b_waits" => {
            play.set_wait(true);
            play.send(B, 0, (&mut squads, &mut field_), content);
            for _ in 0..WAITS_TICKS {
                play.step(&mut squads, &mut field_, content, &mut log);
                lost.count(&log.take(), &field_);
            }
            play.send(A, 0, (&mut squads, &mut field_), content);
        }
        _ => {
            play.send_together(&[A, B], (&mut squads, &mut field_), content);
        }
    }
    for tick in 0..LIMIT_TICKS {
        play.step(&mut squads, &mut field_, content, &mut log);
        lost.count(&log.take(), &field_);
        let living = squads.living(line, &field_.columns.state);
        if living.is_empty() || squads.state[line] == locks::ROUTING {
            return Some(Trial { lost: lost.by_side, winner: Some(0), ticks: tick + 1 });
        }
    }
    Some(Trial { lost: lost.by_side, winner: Some(1), ticks: LIMIT_TICKS })
}

/// BattleTrials._count: a unit lost left the fight (killed outright, downed or surrendered,
/// Decision 121), counted once for its squad's side though it may come to and fall again
/// (Decision 126).
#[derive(Default)]
struct Lost {
    by_side: [i64; 2],
    seen: HashSet<i64>,
}

impl Lost {
    fn count(&mut self, flat: &[i64], field: &Field) {
        for at in starts(flat) {
            if ![DIED, DOWNED, SURRENDERED].contains(&(flat[at] as usize)) {
                continue;
            }
            let unit = flat[at + 4]; // the kind's one field, after kind, tick, squad, faction
            if !self.seen.insert(unit) {
                continue;
            }
            let faction = field.lexicon.word(flat[at + 3] as u32);
            if let Some(side) = FACTIONS.iter().position(|&f| f == faction) {
                self.by_side[side] += 1;
            }
        }
    }
}

/// BattleTrials.summary: the runs, the wins (FACTIONS', then none), each side's losses'
/// spread and the mean ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub runs: usize,
    pub wins: [i64; 3],
    pub lost: [Spread; 2],
    pub ticks: f64,
}

/// [mean, standard deviation, least, most], as BattleTrials._spread gives them.
pub type Spread = [f64; 4];

pub fn summary(results: &[Trial]) -> Summary {
    let mut wins = [0; 3];
    let mut ticks = 0.0;
    for result in results {
        wins[result.winner.unwrap_or(2)] += 1;
        ticks += result.ticks as f64;
    }
    let lost =
        [0, 1].map(|f| spread(&results.iter().map(|r| r.lost[f] as f64).collect::<Vec<_>>()));
    Summary { runs: results.len(), wins, lost, ticks: ticks / results.len().max(1) as f64 }
}

/// BattleTrials._spread, summed in the same order.
fn spread(values: &[f64]) -> Spread {
    if values.is_empty() {
        return [0.0; 4];
    }
    let n = values.len() as f64;
    let mean = values.iter().fold(0.0, |a, v| a + v) / n;
    let square = values.iter().fold(0.0, |a, v| a + (v - mean) * (v - mean));
    let least = values.iter().copied().fold(f64::INFINITY, f64::min);
    let most = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    [mean, (square / n).sqrt(), least, most]
}

/// formation_trials.gd's first line: `runs` as it was given, and the options as Godot prints
/// its dictionary.
pub fn heading(runs: &str, options: &Options) -> String {
    format!(
        "runs {runs}, options {{ \"captain\": {}, \"swap\": {}, \"first_seed\": {}, \"rolls\": {} }}",
        options.captain, options.swap, options.first_seed, options.rolls
    )
}

/// formation_trials.gd's column heads.
pub fn columns() -> String {
    format!(
        "{:<16} {:<22} {:<24} {:<24} {}",
        "scenario", "wins (player/kingdom)", "player lost", "kingdom lost", "ticks"
    )
}

/// formation_trials.gd's `_row`: `"%-16s %-22s %-24s %-24s %.0f"`.
pub fn row(name: &str, summary: &Summary) -> String {
    let [player, kingdom, none] = summary.wins;
    let wins = format!("{player} / {kingdom} ({none} none)");
    let [p, k] = &summary.lost;
    format!("{name:<16} {wins:<22} {:<24} {:<24} {:.0}", shown(p), shown(k), summary.ticks)
}

/// formation_trials.gd's `_spread`: `"%.1f ± %.1f (%d-%d)"`.
fn shown(spread: &Spread) -> String {
    let [mean, sd, least, most] = *spread;
    format!("{mean:.1} ± {sd:.1} ({}-{})", least as i64, most as i64)
}
