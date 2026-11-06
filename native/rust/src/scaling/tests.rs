use super::*;
use crate::content::tests::repo_content;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// tools/scaling_check.gd's `_ready_for` in Godot at ranks 8, stack 16 and settle 20 (seed 1
/// and 2 for each): the stage, the size, the seed, the tick six ticks after it was ready, and how many events
/// each of the next six ticks told of. Since spec 42 step 2 (Decision 161) the crate's own.
const GODOT_S: [(&str, usize, i64, i64, [usize; 6]); 12] = [
    ("march", 16, 1, 9, [0, 0, 0, 0, 0, 0]),
    ("march", 16, 2, 9, [0, 0, 0, 0, 0, 0]),
    ("march", 32, 1, 9, [0, 0, 0, 0, 0, 0]),
    ("march", 32, 2, 9, [0, 0, 0, 0, 0, 0]),
    ("fight", 16, 1, 101, [0, 0, 0, 0, 1, 0]),
    ("fight", 16, 2, 101, [0, 0, 0, 0, 0, 0]),
    ("fight", 32, 1, 101, [0, 0, 0, 0, 0, 1]),
    ("fight", 32, 2, 101, [0, 0, 0, 0, 2, 1]),
    ("stacks", 1, 1, 116, [9, 0, 12, 2, 0, 1]),
    ("stacks", 1, 2, 116, [11, 2, 22, 0, 0, 0]),
    ("stacks", 2, 1, 116, [20, 2, 34, 2, 0, 1]),
    ("stacks", 2, 2, 116, [20, 2, 34, 2, 0, 1]),
];

#[test]
fn each_stage_is_ready_on_the_tick_godot_s_is_and_tells_of_as_much() {
    let content = content();
    let options = Options { stack: 16, ..Options::default() };
    for (stage, size, seed, tick, told) in GODOT_S {
        let mut ready = ready(stage, size, seed, &options, &content).expect("it sets up");
        let mut counts = [0; 6];
        for count in &mut counts {
            *count = ready.step(&content);
        }
        assert_eq!((ready.play.tick, counts), (tick, told), "{stage} {size} {seed}");
    }
}

#[test]
fn the_stacks_are_one_pair_a_lane_each_on_its_own_route() {
    let content = content();
    let options = Options { stack: 16, ..Options::default() };
    let ready = ready("stacks", 2, 1, &options, &content).expect("it sets up");
    assert_eq!(ready.squads.count(), 4);
    assert_eq!(ready.squads.route_code[0], ready.squads.route_code[1]);
    assert_ne!(ready.squads.route_code[0], ready.squads.route_code[2]);
}

#[test]
fn the_lines_are_scaling_check_s_and_a_steep_doubling_fails() {
    let (fight, failures) = lines("fight", &[128, 256, 512], &[1.0, 2.0, 5.5], 2.5);
    let want = [
        "  fight    128 a side      1.00 ms  ",
        "  fight    256 a side      2.00 ms  x2.00",
        "  fight    512 a side      5.50 ms  x2.75  TOO STEEP (limit 2.5)",
    ];
    assert_eq!((fight, failures), (want.map(String::from).to_vec(), 1));
    let (stacks, failures) = lines("stacks", &[1, 2], &[0.5, 1.0], 2.5);
    assert_eq!(stacks[1], "  stacks     2 stacks      1.00 ms  x2.00");
    assert_eq!(failures, 0);
}

#[test]
fn the_least_of_a_stage_is_timed_on_its_battle() {
    let options = Options { runs: 2, ticks: 2, ..Options::default() };
    let ms = least("march", 16, &options, &content()).expect("it sets up");
    assert!(ms.is_finite() && ms >= 0.0);
    assert_eq!(sizes("stacks", &options), [1, 2, 4]);
}
