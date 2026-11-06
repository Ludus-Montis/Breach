use super::*;
use crate::phases::tests::content;
use crate::phases::{settled, Options};

fn with_members(members: Vec<Vec<i32>>) -> Squads {
    Squads { members, ..Squads::default() }
}

#[test]
fn squads_each_on_a_run_of_consecutive_slots_are_packed() {
    let scatter = measured(&with_members(vec![vec![2, 0, 1], vec![3, 4, 5]]));
    assert_eq!(scatter, Scatter { squads: 2, members: 6, runs: 2, span: 6 });
}

#[test]
fn squads_whose_slots_interleave_are_as_scattered_as_they_can_be() {
    let scatter = measured(&with_members(vec![vec![0, 2, 4], vec![1, 3, 5]]));
    assert_eq!(scatter, Scatter { squads: 2, members: 6, runs: 6, span: 10 });
}

#[test]
fn a_squad_with_no_members_isnt_counted() {
    let scatter = measured(&with_members(vec![vec![], vec![7, 8]]));
    assert_eq!(scatter, Scatter { squads: 1, members: 2, runs: 1, span: 2 });
}

#[test]
fn the_line_gives_the_runs_and_the_span_beside_what_packed_would_be() {
    let line = line("at the start", &Scatter { squads: 2, members: 6, runs: 6, span: 10 });
    assert_eq!(
        line,
        "scatter at the start: 2 squads, 6 members in 6 runs (2 packed), spanning 10 slots (6 packed)"
    );
}

#[test]
fn a_settled_battles_squads_are_measured_over_every_member() {
    let options = Options { side: 64, width: 8, squads: 2, settle: 1, ..Options::default() };
    let battle = settled(&options, &content()).expect("the lanes set up");
    let scatter = measured(&battle.squads);
    let members: usize = battle.squads.members.iter().map(Vec::len).sum();
    assert_eq!(scatter.members, members);
    assert!(scatter.squads >= 4, "two lanes, a squad each side: {scatter:?}");
    assert!(scatter.runs >= scatter.squads);
}
