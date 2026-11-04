use super::*;
use crate::phases::tests::content;
use crate::phases::{settled, Options};

fn small() -> Options {
    Options { side: 64, width: 8, settle: 0, ..Options::default() }
}

#[test]
fn a_series_times_each_tick_from_where_it_was_settled_up_to_the_limit() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let from = battle.play.tick;
    let series = timed(&mut battle, 5, &content);
    assert_eq!(series.ticks, (from + 1..=from + 5).collect::<Vec<_>>());
    assert!(series.spent.iter().all(|(_, usec)| usec.len() == 5), "a phase missed a tick");
    assert_eq!(series.spent.len(), crate::phases::tests::PHASES.len());
}

#[test]
fn a_series_stops_once_one_side_no_longer_stands() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let series = timed(&mut battle, LIMIT, &content);
    assert!(series.ticks.len() < LIMIT, "the clash lasted {} ticks", series.ticks.len());
    assert!(!battle.play.both_stand(&battle.squads, &battle.field), "both sides still stand");
    let mut plain = settled(&small(), &content).expect("the clash sets up");
    for _ in 0..series.ticks.len() - 1 {
        plain.step(&content);
    }
    assert!(plain.play.both_stand(&plain.squads, &plain.field), "it timed a tick too many");
}

#[test]
fn a_spread_is_the_mean_the_nearest_rank_percentiles_and_the_first_worst_tick() {
    let values: Vec<f64> = (1..=200).map(|v| f64::from(v % 100 + 1)).collect();
    let ticks: Vec<i64> = (1000..1200).collect();
    let expected = Spread { mean: 50.5, p95: 95.0, p99: 99.0, worst: 100.0, at: 1098 };
    assert_eq!(spread(&values, &ticks), expected);
    let one = Spread { mean: 3.0, p95: 3.0, p99: 3.0, worst: 3.0, at: 7 };
    assert_eq!(spread(&[3.0], &[7]), one);
}

#[test]
fn the_series_lines_put_the_tick_first_and_the_phases_largest_mean_first() {
    let series = Series {
        ticks: vec![41, 42],
        spent: vec![("a", vec![1000.0, 3000.0]), ("b", vec![6000.0, 2000.0])],
    };
    let expected = [
        "series: 2 ticks from tick 41 (ms)      mean     95th     99th    worst  at tick",
        "  tick                                 6.00     7.00     7.00     7.00       41",
        "  b                                    4.00     6.00     6.00     6.00       41",
        "  a                                    2.00     3.00     3.00     3.00       42",
    ];
    assert_eq!(lines(&series), expected);
    assert_eq!(series.sums(), vec![("a", 4000.0), ("b", 8000.0)]);
}
