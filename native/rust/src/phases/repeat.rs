//! tick_phases' repeat mode (spec 47 step 1, Decision 177): `at=<t>[,<t>...] repeat=<n>`
//! replays chosen ticks live, many times, to time a phase under a full tick's pressure with
//! less noise than whole runs give.
//!
//! The battle plays forward once, and just before each tick asked for, everything that tick
//! reads is kept (`Kept`): the state as a snapshot writes it, the play's tick and listed
//! squads, and the field's re-forms (not state, but read). Each tick is then stepped `n` times
//! on the running battle, the copy put back before each (untimed). The field's kept buffers
//! aren't replaced, so they and the allocator stay warm, as in a real battle. One untimed
//! repeat comes first, so the first timed one isn't the cold one. Every lap of every repeat is
//! timed, and for the tick and each phase the lines give the median, the quartiles and a 95%
//! interval of the median; `lap_lines` gives every lap, for the raw output (spec 48 step 5f).

#[cfg(test)]
mod tests;

use super::series::{self, Series};
use super::Settled;
use crate::content::Content;
use crate::cuts;
use crate::laps;
use crate::snapshot;
use crate::swaps::pass::Reforms;

/// `repeat`'s default: how many times each tick is timed.
pub const REPEATS: usize = 50;

/// Everything a tick reads, kept from just before it.
#[derive(Clone)]
pub struct Kept {
    /// The tick it was kept for: the next one stepped.
    pub tick: i64,
    /// The field and the squads as a snapshot writes them, with the play's tick and seed.
    pub(crate) bytes: Vec<u8>,
    listed: Vec<usize>,
    reforms: Reforms,
    /// The state's print (`cuts::print`) once the tick has been played through.
    through: String,
}

impl Kept {
    /// What `settled` reads on its next tick.
    pub fn of(settled: &Settled) -> Kept {
        let play = &settled.play;
        Kept {
            tick: play.tick + 1,
            bytes: snapshot::write(play.tick, play.seed, &settled.field, &settled.squads),
            listed: play.listed.clone(),
            reforms: settled.field.reforms.clone(),
            through: String::new(),
        }
    }

    /// The copy put back into `settled`'s running field, squads and play.
    pub fn put_back(&self, settled: &mut Settled) {
        let state = snapshot::read(&self.bytes).expect("a kept state reads back");
        settled.field.put_back(state.field);
        settled.field.reforms.clone_from(&self.reforms);
        settled.squads = state.squads;
        settled.play.tick = state.tick;
        settled.play.listed.clone_from(&self.listed);
    }
}

/// One tick's repeats: each phase's microseconds on each, and whether the first and the last
/// left the state playing through it once did.
pub struct Repeated {
    pub tick: i64,
    pub laps: Series,
    pub same: bool,
}

/// The middle of some measures: the median, the quartiles (nearest rank) and a 95% interval
/// of the median (the order statistics either side of it, by the binomial's normal
/// approximation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Middle {
    pub median: f64,
    pub q1: f64,
    pub q3: f64,
    pub low: f64,
    pub high: f64,
}

/// Plays `settled` forward once to the last of `ats`, keeping each tick's copy as it passes,
/// in the order given. Refused for a tick it has already passed or one past the battle's end
/// (once the field settles, where a series ends: `series::stirs`), naming the first or the
/// last tick it could repeat.
pub fn kept(settled: &mut Settled, ats: &[i64], content: &Content) -> Result<Vec<Kept>, String> {
    let first = settled.play.tick + 1;
    if let Some(early) = ats.iter().find(|&&t| t < first) {
        return Err(format!(
            "at={early} comes before the battle is settled: its first tick is {first}"
        ));
    }
    let mut ahead: Vec<i64> = ats.to_vec();
    ahead.sort_unstable();
    ahead.dedup();
    let mut copies: Vec<Kept> = Vec::new();
    for t in ahead {
        while settled.play.tick < t {
            if !series::stirs(settled) {
                let last = settled.play.tick;
                return Err(format!("at={t} is past the battle's end: its last tick is {last}"));
            }
            let keeping = settled.play.tick + 1 == t;
            let copy = keeping.then(|| Kept::of(settled));
            settled.step(content);
            if let Some(mut copy) = copy {
                copy.through = cuts::print(&settled.field, &settled.squads);
                copies.push(copy);
            }
        }
    }
    let find = |t: &i64| copies.iter().find(|k| k.tick == *t).expect("each tick was kept");
    Ok(ats.iter().map(|t| find(t).clone()).collect())
}

/// `kept`'s tick stepped `n` times (at least once) on `settled`, the copy put back before
/// each, after one untimed repeat; every lap timed.
pub fn repeat(settled: &mut Settled, kept: &Kept, n: usize, content: &Content) -> Repeated {
    let n = n.max(1);
    kept.put_back(settled);
    settled.step(content);
    let mut repeated = Repeated { tick: kept.tick, laps: Series::default(), same: true };
    for i in 0..n {
        kept.put_back(settled);
        let (_, laps) = laps::timed(|| settled.step(content));
        repeated.laps.push(kept.tick, laps);
        if i == 0 || i == n - 1 {
            repeated.same &= cuts::print(&settled.field, &settled.squads) == kept.through;
        }
    }
    repeated
}

/// The middle of `values`; all 0 for none.
pub fn middle(values: &[f64]) -> Middle {
    if values.is_empty() {
        return Middle { median: 0.0, q1: 0.0, q3: 0.0, low: 0.0, high: 0.0 };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    // 1-based ranks, clamped to the values there are
    let at = |rank: f64| sorted[(rank as usize).clamp(1, n) - 1];
    let nearest = |p: usize| at((p * n).div_ceil(100) as f64);
    let median = (sorted[(n - 1) / 2] + sorted[n / 2]) / 2.0;
    let half = 1.96 * (n as f64).sqrt() / 2.0;
    let low = at((n as f64 / 2.0 - half).floor().max(1.0));
    let high = at((1.0 + n as f64 / 2.0 + half).ceil());
    Middle { median, q1: nearest(25), q3: nearest(75), low, high }
}

/// The lines for one repeated tick: a `repeat:` line naming it and `n`, then the medians on
/// the lines `phases::lines` prints (so `bench_compare.py` reads them as a run's), then each
/// one's middle (median, quartiles, interval), the tick first and the phases largest median first. Those end in a bracket,
/// so `bench_compare.py` doesn't read them as phases.
pub fn lines(battle: &str, repeated: &Repeated) -> Vec<String> {
    let laps = &repeated.laps;
    let (t, n) = (repeated.tick, laps.ticks.len());
    let ms = |usec: f64| usec / 1000.0;
    let tick = middle(&laps.totals());
    let phases: Vec<(&str, Middle)> =
        largest_first(laps).into_iter().map(|(phase, usec)| (phase, middle(usec))).collect();
    let mut out = vec![
        format!("repeat: tick {t}, {n} repeats"),
        format!("{battle}, tick {t}: {:.2} ms a tick", ms(tick.median)),
    ];
    out.extend(phases.iter().map(|(name, m)| format!("  {:<32} {:>8.2}", name, ms(m.median))));
    out.push(format!(
        "  {:<32} {:>8} {:>8} {:>8}   {}",
        "(ms)", "median", "q1", "q3", "95% interval of the median"
    ));
    for (name, m) in [("tick", tick)].into_iter().chain(phases) {
        out.push(format!(
            "  {:<32} {:>8.2} {:>8.2} {:>8.2}   [{:.2}, {:.2}]",
            name,
            ms(m.median),
            ms(m.q1),
            ms(m.q3),
            ms(m.low),
            ms(m.high)
        ));
    }
    out
}

/// The `laps=1` block after a repeated tick's table (spec 48 step 5f): a `laps:` line, then
/// every lap in ms, in the order run, the tick first and the phases in the table's order. Each
/// list ends in a bracket, so the table's readers skip it as they skip the middles.
pub fn lap_lines(repeated: &Repeated) -> Vec<String> {
    let laps = &repeated.laps;
    let listed = |usec: &[f64]| {
        let ms: Vec<String> = usec.iter().map(|u| format!("{:.3}", u / 1000.0)).collect();
        ms.join(", ")
    };
    let totals = laps.totals();
    let rows = [("tick", &totals)].into_iter().chain(largest_first(laps));
    let mut out = vec!["laps:".to_string()];
    out.extend(rows.map(|(name, usec)| format!("  {:<32} [{}]", name, listed(usec))));
    out
}

/// Each phase's laps, the largest median first, as the table lists them.
fn largest_first(laps: &Series) -> Vec<(&'static str, &Vec<f64>)> {
    let mut phases: Vec<(&'static str, &Vec<f64>, f64)> =
        laps.spent.iter().map(|(phase, usec)| (*phase, usec, middle(usec).median)).collect();
    phases.sort_by(|a, b| b.2.total_cmp(&a.2));
    phases.into_iter().map(|(phase, usec, _)| (phase, usec)).collect()
}
