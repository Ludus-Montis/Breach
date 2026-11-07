//! FatigueTrials' battles, sums and lines as formation_trials.gd's `fatigue` fights, sums and
//! prints them.

use super::*;
use crate::content::tests::repo_content;
use crate::threads::Threads;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

#[test]
fn the_lines_are_printed_as_formation_trials_prints_them() {
    assert_eq!(columns(), "contact    stamina    pursued (cells)  ticks        tired");
    let result = Fatigue { pursued: 12.25, ticks: 40.04, stamina: 0.875, tired: 1, runs: 2 };
    assert_eq!(row(30.0, &result), "30 s       88%        12.2             40.0         1 / 2");
}

#[test]
fn the_runs_equal_godot_s() {
    // `godot --headless --path . --script res://tools/formation_trials.gd -- scenario=fatigue
    // runs=2`, its lines after the heads.
    let content = content();
    let rows: Vec<String> = CONTACTS
        .iter()
        .map(|&seconds| row(seconds, &run(seconds, 2, 1, Threads::One, &content).expect("a grem")))
        .collect();
    assert_eq!(rows, GODOT);
}

#[test]
fn a_battle_is_the_same_threaded_or_not() {
    let content = content();
    let alone = battle(30.0, 4, Threads::One, &content);
    assert_eq!(battle(30.0, 4, Threads::Always, &content), alone);
}

/// Godot's lines for `scenario=fatigue runs=2`; since spec 42 step 2 (Decision 161) the
/// crate's own.
const GODOT: [&str; 4] = [
    "3 s        97%        127.0            181.5        0 / 2",
    "30 s       70%        127.0            196.0        0 / 2",
    "60 s       40%        49.4             80.0         2 / 2",
    "90 s       10%        0.0              0.0          2 / 2",
];
