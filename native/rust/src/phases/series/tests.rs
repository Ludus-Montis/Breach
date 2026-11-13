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
    assert_eq!(series.spent.len(), crate::phases::tests::PARTS.len());
}

#[test]
fn a_series_runs_past_the_break_until_no_squad_fights_or_routes() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let series = timed(&mut battle, LIMIT, &content);
    assert!(series.ticks.len() < LIMIT, "the field never settled in {} ticks", series.ticks.len());
    assert!(!stirs(&battle), "a squad still fights or routes");
    let mut plain = settled(&small(), &content).expect("the clash sets up");
    for _ in 0..series.ticks.len() - 1 {
        plain.step(&content);
    }
    assert!(stirs(&plain), "it timed a tick too many");
    assert!(!plain.play.both_stand(&plain.squads, &plain.field), "it ended before one side broke");
}

#[test]
fn inspect_names_the_3_slowest_and_4_spread_ticks_in_order() {
    // Ticks 10 to 22; the slowest are 15 (9), then 12 and 20 (8, the earlier first), then 21.
    let usec = [1.0, 2.0, 8.0, 3.0, 4.0, 9.0, 1.0, 2.0, 3.0, 4.0, 8.0, 7.0, 5.0];
    let series = Series { ticks: (10..23).collect(), spent: vec![("a", usec.to_vec())], cpu: None };
    assert_eq!(inspect(&series), vec![10, 12, 14, 15, 18, 20, 22]);
    assert_eq!(inspect_line(&series), "inspect: 10,12,14,15,18,20,22");
}

#[test]
fn a_tick_picked_twice_gives_its_place_to_the_next_slowest() {
    // The slowest are the first and last ticks (spread), so 3 more slowest join them.
    let usec = [9.0, 1.0, 5.0, 1.0, 1.0, 6.0, 4.0, 1.0, 3.0, 1.0, 9.0];
    let series = Series { ticks: (0..11).collect(), spent: vec![("a", usec.to_vec())], cpu: None };
    assert_eq!(inspect(&series), vec![0, 2, 3, 5, 6, 8, 10]);
    let short = Series { ticks: vec![4, 5, 6], spent: vec![("a", vec![1.0, 2.0, 3.0])], cpu: None };
    assert_eq!(inspect(&short), vec![4, 5, 6]);
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
fn the_series_lines_put_the_tick_first_the_phases_largest_mean_first_then_inspect() {
    let series = Series {
        ticks: vec![41, 42],
        spent: vec![("a", vec![1000.0, 3000.0]), ("b", vec![6000.0, 2000.0])],
        cpu: None,
    };
    let expected = [
        "series: 2 ticks from tick 41 (ms)      mean     95th     99th    worst  at tick",
        "  tick                                 6.00     7.00     7.00     7.00       41",
        "  b                                    4.00     6.00     6.00     6.00       41",
        "  a                                    2.00     3.00     3.00     3.00       42",
        "inspect: 41,42",
    ];
    assert_eq!(lines(&series), expected);
    assert_eq!(series.sums(), vec![("a", 4000.0), ("b", 8000.0)]);
}
