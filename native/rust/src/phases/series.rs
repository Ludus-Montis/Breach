//! tick_phases' `series=1` in the crate (spec 38 step 2, built in spec 39 step 8c-xx): every
//! tick of a battle timed from where `settled` leaves it until the field settles (no squad
//! fights or routes, so the rout is inside it; spec 47 step 2a, Decision 177) or `limit` ticks,
//! and for the tick and each phase the mean, the 95th and 99th percentiles and the worst, with
//! the tick that was worst. The means go on the lines `phases::lines` prints, so
//! `bench_compare.py` reads them as it reads any run; the rest follow a "series:" line, which it
//! reads when every run has one. An "inspect:" line ends them: the ticks the repeat mode
//! replays (its 3 slowest and 4 spread through it).

#[cfg(test)]
mod tests;

use super::Settled;
use crate::content::Content;
use crate::laps;
use crate::locks::{FIGHTING, ROUTING};

/// tick_phases' `limit`: the most ticks a series times.
pub const LIMIT: usize = 600;

/// How many of a series' slowest ticks `inspect` names, beside its 4 spread ones.
const SLOWEST: usize = 3;

/// A battle's timed ticks: the tick each was, and each phase's microseconds on each of them
/// (0 on a tick it didn't lap), the phases in the order first lapped.
#[derive(Debug, Default)]
pub struct Series {
    pub ticks: Vec<i64>,
    pub spent: Vec<(&'static str, Vec<f64>)>,
}

impl Series {
    /// Each phase's microseconds summed over the series, as `phases::timed` gives them.
    pub fn sums(&self) -> Vec<(&'static str, f64)> {
        self.spent.iter().map(|(phase, usec)| (*phase, usec.iter().sum())).collect()
    }

    /// `tick` added, with each phase's microseconds lapped on it (0 for a phase it didn't lap).
    pub fn push(&mut self, tick: i64, laps: Vec<(&'static str, f64)>) {
        let before = self.ticks.len();
        self.ticks.push(tick);
        for (phase, usec) in laps {
            match self.spent.iter_mut().find(|(name, _)| *name == phase) {
                Some((_, spent)) => spent.push(usec),
                None => self.spent.push((phase, [vec![0.0; before], vec![usec]].concat())),
            }
        }
        for (_, spent) in &mut self.spent {
            spent.resize(before + 1, 0.0);
        }
    }

    /// The whole tick's microseconds on each tick: its phases' summed.
    pub fn totals(&self) -> Vec<f64> {
        (0..self.ticks.len()).map(|t| self.spent.iter().map(|(_, usec)| usec[t]).sum()).collect()
    }
}

/// One measure's spread over a series, in microseconds: its mean, 95th and 99th percentiles
/// (nearest rank), its worst and the first tick that was worst.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spread {
    pub mean: f64,
    pub p95: f64,
    pub p99: f64,
    pub worst: f64,
    pub at: i64,
}

/// Steps `settled` a tick at a time, each timed, until the field settles (no squad fights or
/// routes; Decision 177) or `limit` ticks have been timed.
pub fn timed(settled: &mut Settled, limit: usize, content: &Content) -> Series {
    let mut series = Series::default();
    while series.ticks.len() < limit && stirs(settled) {
        let (_, laps) = laps::timed(|| settled.step(content));
        series.push(settled.play.tick, laps);
    }
    series
}

/// Whether the field hasn't settled: some listed squad with a living unit still fights or routes.
pub fn stirs(settled: &Settled) -> bool {
    let (squads, states) = (&settled.squads, &settled.field.columns.state);
    settled.play.listed.iter().any(|&s| {
        matches!(squads.state[s], FIGHTING | ROUTING) && !squads.living(s, states).is_empty()
    })
}

/// The series' inspection ticks (Decision 177), earliest first: its first tick, those a third
/// and two thirds of the way and its last, then its slowest ticks (the earlier on a tie) not
/// already picked, until there are 7 or every tick is.
pub fn inspect(series: &Series) -> Vec<i64> {
    let n = series.ticks.len();
    let mut picked: Vec<usize> = Vec::new();
    for at in (0..4).map(|k| k * n.saturating_sub(1) / 3) {
        if n > 0 && !picked.contains(&at) {
            picked.push(at);
        }
    }
    let totals = series.totals();
    let mut slowest: Vec<usize> = (0..n).collect();
    slowest.sort_by(|&a, &b| totals[b].total_cmp(&totals[a]).then(a.cmp(&b)));
    for at in slowest {
        if picked.len() >= n.min(SLOWEST + 4) {
            break;
        }
        if !picked.contains(&at) {
            picked.push(at);
        }
    }
    picked.sort_unstable();
    picked.into_iter().map(|at| series.ticks[at]).collect()
}

/// The "inspect:" line: the inspection ticks, comma-separated as `at=` takes them.
pub fn inspect_line(series: &Series) -> String {
    let ticks: Vec<String> = inspect(series).iter().map(i64::to_string).collect();
    format!("inspect: {}", ticks.join(","))
}

/// The spread of `values`, one a tick of `ticks`; all 0 for none.
pub fn spread(values: &[f64], ticks: &[i64]) -> Spread {
    if values.is_empty() {
        return Spread { mean: 0.0, p95: 0.0, p99: 0.0, worst: 0.0, at: 0 };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |p: usize| sorted[((p * sorted.len()).div_ceil(100)).clamp(1, sorted.len()) - 1];
    let worst = sorted[sorted.len() - 1];
    let at = values.iter().position(|&v| v == worst).map_or(0, |t| ticks[t]);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    Spread { mean, p95: rank(95), p99: rank(99), worst, at }
}

/// The lines that follow `phases::lines` in a series run: the "series:" line, the tick's
/// spread and each phase's in ms, largest mean first, then the "inspect:" line.
pub fn lines(series: &Series) -> Vec<String> {
    let ticks = &series.ticks;
    let first = ticks.first().copied().unwrap_or(0);
    let head = format!("series: {} ticks from tick {first} (ms)", ticks.len());
    let mut out = vec![format!(
        "{head:<34} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "mean", "95th", "99th", "worst", "at tick"
    )];
    let mut rows = vec![("tick", spread(&series.totals(), ticks))];
    let mut phases: Vec<_> =
        series.spent.iter().map(|(phase, usec)| (*phase, spread(usec, ticks))).collect();
    phases.sort_by(|a, b| b.1.mean.total_cmp(&a.1.mean));
    rows.extend(phases);
    for (name, s) in rows {
        let ms = |usec: f64| usec / 1000.0;
        out.push(format!(
            "  {:<32} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8}",
            name,
            ms(s.mean),
            ms(s.p95),
            ms(s.p99),
            ms(s.worst),
            s.at
        ));
    }
    out.push(inspect_line(series));
    out
}
