//! tick_phases' `series=1` in the crate (spec 38 step 2, built in spec 39 step 8c-xx): every
//! tick of a battle timed from where `settled` leaves it until one side no longer stands or
//! `limit` ticks, and for the tick and each phase the mean, the 95th and 99th percentiles
//! and the worst, with the tick that was worst. The means go on the lines `phases::lines`
//! prints, so `bench_compare.py` reads them as it reads any run; the rest follow a "series:"
//! line, which it reads when every run has one.

#[cfg(test)]
mod tests;

use super::Settled;
use crate::content::Content;
use crate::laps;

/// tick_phases' `limit`: the most ticks a series times.
pub const LIMIT: usize = 600;

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

/// Steps `settled` a tick at a time, each timed, while both sides stand and fewer than
/// `limit` ticks have been timed.
pub fn timed(settled: &mut Settled, limit: usize, content: &Content) -> Series {
    let mut series = Series::default();
    while series.ticks.len() < limit && settled.play.both_stand(&settled.squads, &settled.field) {
        let (_, laps) = laps::timed(|| settled.step(content));
        let before = series.ticks.len();
        series.ticks.push(settled.play.tick);
        for (phase, usec) in laps {
            match series.spent.iter_mut().find(|(name, _)| *name == phase) {
                Some((_, spent)) => spent.push(usec),
                None => series.spent.push((phase, [vec![0.0; before], vec![usec]].concat())),
            }
        }
        for (_, spent) in &mut series.spent {
            spent.resize(before + 1, 0.0);
        }
    }
    series
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

/// The lines that follow `phases::lines` in a series run: the "series:" line, then the tick's
/// spread and each phase's in ms, largest mean first.
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
    out
}
