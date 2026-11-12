use super::*;
use crate::phases::tests::{content, PHASES};
use crate::phases::{settled, Options};

/// The clash at side 64, width 8, three ticks past contact.
fn small() -> Options {
    Options { side: 64, width: 8, settle: 3, ..Options::default() }
}

/// The same clash at tick 0, unstepped: the march from the start.
fn from_the_start() -> Options {
    Options { march: true, settle: 0, ..small() }
}

#[test]
fn a_counted_battle_ends_where_an_uncounted_one_does() {
    let content = content();
    let mut counted_one = settled(&small(), &content).expect("the clash sets up");
    let mut plain = settled(&small(), &content).expect("the clash sets up");
    counted(&mut counted_one, 4, &content);
    for _ in 0..4 {
        plain.step(&content);
    }
    assert_eq!(counted_one.play.tick, plain.play.tick);
    assert!(counted_one.state() == plain.state(), "counting changed the battle");
}

#[test]
fn each_pass_is_counted_in_the_order_the_tick_runs_them() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let counts = counted(&mut battle, 2, &content);
    let names: Vec<&str> = counts.passes.iter().map(|count| count.phase).collect();
    assert_eq!(counts.ticks, 2);
    assert_eq!(names, PHASES);
}

#[test]
fn on_the_march_every_pass_counts_both_sides_living_and_the_rules_change_nothing() {
    let content = content();
    let mut battle = settled(&from_the_start(), &content).expect("the clash sets up");
    let counts = counted(&mut battle, 1, &content);
    let each: Vec<(&str, usize)> =
        counts.passes.iter().map(|c| (c.phase, c.living / c.laps.max(1))).collect();
    let both_sides: Vec<(&str, usize)> = PHASES.iter().map(|phase| (*phase, 128)).collect();
    assert_eq!(each, both_sides);
    let changed = |phase: &str| counts.passes.iter().find(|c| c.phase == phase).map(|c| c.changed);
    assert_eq!(changed("rules"), Some(0));
    assert!(changed("units walk to places").is_some_and(|n| n > 0), "the march moved no one");
}

#[test]
fn a_slot_is_changed_when_any_of_its_columns_differs_or_only_one_side_has_it() {
    let content = content();
    let battle = settled(&from_the_start(), &content).expect("the clash sets up");
    let before = battle.field.columns.clone();
    let mut after = before.clone();
    after.hp[3] += 1;
    after.sheet[7] = after.sheet[0].clone();
    after.sheet[7].courage += 1;
    after.bearing[9] += 1.0;
    let added = after.state.len();
    after.state.push(0);
    assert_eq!(changed(&before, &before.clone()), Vec::<usize>::new());
    assert_eq!(changed(&before, &after), vec![3, 7, 9, added]);
}

#[test]
fn the_lines_give_each_pass_a_lap_s_changed_against_the_living_in_the_tick_s_order() {
    let counts = Counts {
        ticks: 2,
        passes: vec![
            Count { phase: "orders", laps: 2, changed: 0, living: 256 },
            Count { phase: "roster", laps: 4, changed: 2, living: 400 },
            Count { phase: "fight (blows)", laps: 2, changed: 64, living: 256 },
        ],
    };
    let expected = [
        "counts: slots changed against the living, a lap, over 2 ticks",
        "  orders                                0.0 of    128.0    0.0%",
        "  roster                                0.5 of    100.0    0.5%",
        "  fight (blows)                        32.0 of    128.0   25.0%",
    ];
    assert_eq!(lines(&counts), expected);
}

/// The small clash's counts over three ticks past contact, held so a change to a pass's
/// writes shows: which passes change every living unit, which change some and which none.
const SMALL_CLASH: &[(&str, usize, usize)] = &[
    ("rules", 3, 0),
    ("roster", 15, 0),
    ("orders", 3, 0),
    ("perception", 3, 0),
    ("engage: fronts", 3, 0),
    ("engage: edges", 3, 0),
    ("pathing", 3, 0),
    ("march (squads move)", 3, 0),
    ("joins", 3, 0),
    ("scrum: engage, prepare, stance", 3, 0),
    ("scrum: facing", 3, 384),
    ("scrum: chase: steer", 3, 0),
    ("scrum: withdraw", 3, 0),
    ("scrum: pursuit, stall", 3, 0),
    ("fight (blows)", 3, 48),
    ("deaths (bury)", 3, 0),
    ("taking the downed", 3, 0),
    ("wounds (tend)", 3, 0),
    ("recovery", 3, 0),
    ("strays", 3, 0),
    ("morale", 3, 0),
    ("rout: listing", 3, 0),
    ("carry", 3, 0),
    ("carry: strays", 3, 0),
    ("groups (form up, split)", 3, 0),
    ("stamina", 3, 384),
    ("bodies: commit", 3, 384),
    ("shuffle (re-form)", 3, 0),
    ("units walk to places", 3, 384),
    ("store settles", 3, 0),
];

#[test]
fn the_small_clash_s_counts_hold() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let counts = counted(&mut battle, 3, &content);
    let got: Vec<(&str, usize, usize, usize)> =
        counts.passes.iter().map(|c| (c.phase, c.laps, c.changed, c.living)).collect();
    let held: Vec<(&str, usize, usize, usize)> = SMALL_CLASH
        .iter()
        .map(|&(phase, laps, changed)| (phase, laps, changed, laps * 128))
        .collect();
    assert_eq!(got, held);
}
