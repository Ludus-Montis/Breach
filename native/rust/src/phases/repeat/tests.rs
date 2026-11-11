use super::*;
use crate::phases::series::{self, LIMIT};
use crate::phases::tests::content;
use crate::phases::{settled, Options};
use crate::snapshot;

fn small() -> Options {
    Options { side: 64, width: 8, settle: 0, ..Options::default() }
}

#[test]
fn a_repeated_tick_leaves_the_state_the_battle_reaches_playing_through_it_once() {
    let content = content();
    let mut plain = settled(&small(), &content).expect("the clash sets up");
    let at = plain.play.tick + 3;
    for _ in 0..3 {
        plain.step(&content);
    }
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let kept = kept(&mut battle, &[at], &content).expect("the tick is in the battle");
    let repeated = repeat(&mut battle, &kept[0], 4, &content);
    assert_eq!((repeated.tick, repeated.laps.ticks.len()), (at, 4));
    assert!(repeated.same, "a repeat left another state than the tick played through");
    assert_eq!(battle.play.tick, plain.play.tick);
    assert!(battle.state() == plain.state(), "the repeats ended apart from the played tick");
    let prints = |b: &Settled| cuts::print(&b.field, &b.squads);
    assert_eq!(prints(&battle), prints(&plain));
}

#[test]
fn the_put_back_keeps_the_buffers_warm_and_is_the_state_a_snapshot_reads_back() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    battle.step(&content);
    let kept = Kept::of(&battle);
    battle.step(&content);
    assert!(battle.field.buffers.warm(), "the tick kept nothing in its buffers");
    kept.put_back(&mut battle);
    assert!(battle.field.buffers.warm(), "the put-back emptied the kept buffers");
    let read = snapshot::read(&kept.bytes).expect("the kept state reads back");
    let written = |field, squads| snapshot::write(0, 0, field, squads);
    assert!(written(&battle.field, &battle.squads) == written(&read.field, &read.squads));
    assert_eq!(battle.play.tick, read.tick);
}

#[test]
fn an_at_past_the_battle_s_end_is_refused_naming_its_last_tick() {
    let content = content();
    let mut whole = settled(&small(), &content).expect("the clash sets up");
    let last = *series::timed(&mut whole, LIMIT, &content).ticks.last().expect("a tick");
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let refused = kept(&mut battle, &[last + 1], &content).err().expect("it was refused");
    assert!(refused.contains(&format!("last tick is {last}")), "{refused}");
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    assert!(kept(&mut battle, &[last], &content).is_ok(), "the last tick was refused");
}

#[test]
fn the_ticks_are_kept_in_the_order_given_from_one_play_forward() {
    let content = content();
    let mut battle = settled(&small(), &content).expect("the clash sets up");
    let from = battle.play.tick;
    let kept = kept(&mut battle, &[from + 3, from + 1], &content).expect("both are in it");
    let ticks: Vec<i64> = kept.iter().map(|k| k.tick).collect();
    assert_eq!(ticks, [from + 3, from + 1]);
    assert_eq!(battle.play.tick, from + 3, "it played past the last tick asked for");
}

#[test]
fn the_middle_is_the_median_the_quartiles_and_the_median_s_95_interval() {
    let values: Vec<f64> = (1..=20).map(f64::from).collect();
    let m = middle(&values);
    assert_eq!((m.median, m.q1, m.q3), (10.5, 5.0, 15.0));
    // ranks 10 - 1.96 * sqrt(20) / 2 and 11 + 1.96 * sqrt(20) / 2, rounded outward
    assert_eq!((m.low, m.high), (5.0, 16.0));
    let one = middle(&[3.0]);
    assert_eq!((one.median, one.q1, one.q3, one.low, one.high), (3.0, 3.0, 3.0, 3.0, 3.0));
}

#[test]
fn the_lines_name_the_tick_then_read_as_a_run_s_then_give_each_middle() {
    let mut laps = Series::default();
    for (a, b) in [(1000.0, 3000.0), (2000.0, 4000.0), (3000.0, 5000.0)] {
        laps.push(105, vec![("a", a), ("b", b)]);
    }
    let repeated = Repeated { tick: 105, laps, same: true };
    let out = lines("64 a side, 8 wide", &repeated);
    let expected = [
        "repeat: tick 105, 3 repeats",
        "64 a side, 8 wide, tick 105: 6.00 ms a tick",
        "  b                                    4.00",
        "  a                                    2.00",
        "  (ms)                               median      IQR   95% interval of the median",
        "  tick                                 6.00     4.00   [4.00, 8.00]",
        "  b                                    4.00     2.00   [3.00, 5.00]",
        "  a                                    2.00     2.00   [1.00, 3.00]",
    ];
    assert_eq!(out, expected);
}
