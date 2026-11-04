//! BattleTrials' runs, summary and lines as formation_trials.gd fights, sums and prints them.

use super::*;
use crate::content::tests::repo_content;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

fn trial(lost: [i64; 2], winner: Option<usize>, ticks: i64) -> Trial {
    Trial { lost, winner, ticks }
}

#[test]
fn a_summary_counts_the_wins_and_spreads_the_losses_as_battle_trials_does() {
    let results =
        [trial([2, 8], Some(0), 300), trial([8, 3], Some(1), 401), trial([0, 0], None, 2000)];
    let summary = summary(&results);
    assert_eq!(summary.runs, 3);
    assert_eq!(summary.wins, [1, 1, 1]);
    let mean = 10.0 / 3.0;
    let sd = (((2.0 - mean) * (2.0 - mean) + (8.0 - mean) * (8.0 - mean) + mean * mean) / 3.0_f64)
        .sqrt();
    assert_eq!(summary.lost[0], [mean, sd, 0.0, 8.0]);
    assert_eq!(summary.lost[1][2..], [0.0, 8.0]);
    assert_eq!(summary.ticks, 2701.0 / 3.0);
    assert_eq!(super::summary(&[]).lost, [[0.0; 4]; 2]);
}

#[test]
fn the_lines_are_printed_as_formation_trials_prints_them() {
    let options = Options { swap: true, ..Options::default() };
    assert_eq!(
        heading("50", &options),
        "runs 50, options { \"captain\": false, \"swap\": true, \"first_seed\": 1, \"rolls\": true }"
    );
    assert_eq!(
        columns(),
        "scenario         wins (player/kingdom)  player lost              kingdom lost             ticks"
    );
    let summary = Summary {
        runs: 2,
        wins: [1, 0, 1],
        lost: [[2.25, 0.25, 2.0, 3.0], [8.0, 0.0, 8.0, 8.0]],
        ticks: 312.5,
    };
    assert_eq!(
        row("mirror_headon", &summary),
        "mirror_headon    1 / 0 (1 none)         2.2 ± 0.2 (2-3)          8.0 ± 0.0 (8-8)          312"
    );
}

#[test]
fn a_mirror_is_fought_till_one_side_no_longer_stands_each_unit_lost_counted_once() {
    let content = content();
    let results = run("mirror_headon", 2, &Options::default(), &content).expect("it is set up");
    assert_eq!(results.len(), 2);
    for result in &results {
        assert!(result.winner.is_some() && result.ticks < LIMIT_TICKS, "{result:?}");
        assert!(result.lost.iter().all(|&n| (0..=8).contains(&n)), "{result:?}");
        assert!(result.lost.iter().sum::<i64>() > 0, "{result:?}");
    }
    assert_ne!(results[0], results[1], "each run its own seed");
    let flank = run("mirror_flank", 1, &Options { swap: true, ..Options::default() }, &content);
    assert!(flank.is_some_and(|r| r[0].winner.is_some()));
}

#[test]
fn a_field_scenario_is_played_till_the_line_breaks_or_the_limit() {
    let content = content();
    let results = run("field_a", 1, &Options::default(), &content).expect("it is set up");
    let result = results[0];
    assert!(result.ticks > 0 && result.ticks <= LIMIT_TICKS, "{result:?}");
    assert!(result.winner.is_some(), "{result:?}");
    assert!(result.lost.iter().sum::<i64>() > 0, "{result:?}");
}

#[test]
fn a_scenario_the_trials_do_not_know_is_none() {
    assert_eq!(run("fatigue", 1, &Options::default(), &content()), None);
}
