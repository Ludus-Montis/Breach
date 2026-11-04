use super::*;
use crate::content::tests::repo_content;

pub(crate) fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

fn small() -> Options {
    Options { side: 64, width: 8, settle: 3, fight: 2, ..Options::default() }
}

/// The phases a tick laps, in the order first lapped: the crate's passes, as tick.rs and its
/// five passes run them.
pub(crate) const PHASES: &[&str] = &[
    "rules",
    "roster",
    "orders",
    "perception",
    "engage: fronts",
    "engage: edges",
    "pathing",
    "march (squads move)",
    "joins",
    "scrum: engage, prepare, stance",
    "scrum: seek, walk and facing",
    "scrum: regroup and chasers",
    "scrum: withdraw",
    "scrum: pursuit, stall",
    "fight (blows)",
    "deaths (bury)",
    "taking the downed",
    "wounds (tend)",
    "recovery",
    "strays",
    "morale",
    "rout",
    "carry",
    "carry: strays",
    "groups (form up, split)",
    "stamina",
    "bodies (parting)",
    "shuffle (re-form)",
    "units walk to places",
    "store settles",
];

#[test]
fn a_tick_laps_each_of_the_crate_s_passes_in_the_order_it_runs_them() {
    let content = content();
    let mut settled = settled(&small(), &content).expect("the clash sets up");
    let spent = timed(&mut settled, 1, &content);
    let names: Vec<&str> = spent.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, PHASES);
}

#[test]
fn a_timed_battle_ends_where_an_untimed_one_does() {
    let content = content();
    let options = Options { retreat: Some(1), ..small() };
    let mut timed_one = settled(&options, &content).expect("the clash sets up");
    let mut plain = settled(&options, &content).expect("the clash sets up");
    timed(&mut timed_one, 4, &content);
    for _ in 0..4 {
        plain.step(&content);
    }
    assert_eq!(timed_one.play.tick, plain.play.tick);
    assert!(timed_one.state() == plain.state(), "the laps changed the battle");
}

#[test]
fn the_lines_put_the_battle_first_and_the_phases_largest_first() {
    let spent = [("a", 2000.0), ("b", 6000.0), ("c", 4000.0)];
    let options = Options { side: 64, width: 8, fight: 2, ..Options::default() };
    let expected = [
        "64 a side, 8 wide: 6.0 ms a tick",
        "  b                                    3.00",
        "  c                                    2.00",
        "  a                                    1.00",
    ];
    assert_eq!(lines(&options, 2, &spent), expected);
}

/// tools/tick_phases.gd's `_settled` in Godot (side 64, width 8): each case's settle, squads,
/// march, retreat, seed and archers a squad, the tick six ticks after it was settled, and how
/// many events each of those six told of.
#[allow(clippy::type_complexity)]
const GODOT_S: [((usize, usize, bool, Option<i64>, i64, usize), i64, [usize; 6]); 5] = [
    ((3, 1, false, None, 1, 0), 99, [8, 2, 0, 0, 0, 0]),
    ((3, 1, false, Some(1), 2, 0), 99, [8, 0, 0, 0, 0, 0]),
    ((10, 2, false, None, 1, 0), 106, [2, 0, 17, 17, 9, 2]),
    ((3, 1, false, None, 1, 2), 99, [6, 2, 0, 0, 0, 0]),
    ((10, 1, true, None, 1, 0), 16, [0, 0, 0, 0, 0, 0]),
];

#[test]
fn each_battle_is_settled_on_the_tick_godot_s_is_and_tells_of_as_much() {
    let content = content();
    for ((settle, squads, march, retreat, seed, archers), tick, counts) in GODOT_S {
        let options = Options { settle, squads, march, retreat, seed, archers, ..small() };
        let mut settled = settled(&options, &content).expect("the battle sets up");
        let told: Vec<usize> = (0..6).map(|_| settled.step(&content)).collect();
        assert_eq!((settled.play.tick, told), (tick, counts.to_vec()), "{options:?}");
    }
}

/// FormationBench.mixed (spec 38 step 2's `terrain=mixed`) as Godot played it from the start,
/// from a throwaway probe (spec 39 step 8c-xxi): (side, width, seed, archers), the events told
/// over its first 400 ticks, the first tick that told of any, and the SHA-256 of
/// DigestState.state at tick 130 (the fight) and at tick 400 (long after it ended).
type Mixed = ((usize, usize, i64, usize), usize, i64, &'static str, &'static str);

const GODOT_MIXED: [Mixed; 3] = [
    (
        (64, 8, 1, 0),
        447,
        108,
        "65322c7b9d200e5243668e5d17d46b154158188607adcb8dbe14ae5880c78cdf",
        "e3b511ff556c4a9389cfd0adab9987089f313426acb734361f683db8154d69bb",
    ),
    (
        (64, 8, 2, 0),
        463,
        108,
        "702f148c53862530aa799ae0f3962f314a6090692e6c0d1942134c5fc1cb6d98",
        "fc3dd57b5862009def0974bd2ada258233de7325062ddb837a58555f6883cf93",
    ),
    (
        (96, 12, 1, 4),
        553,
        108,
        "c31f47f8d9b5ffdc1d38e15302821453af207f39ea5aa5115a37a2a6eccd99aa",
        "b26640a5cd0b3e8f59ed3029d60673ab11a0ab866afda16284104518ecdb68ea",
    ),
];

fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_mixed_clash_plays_as_godot_s_did_and_its_digest_is_the_same() {
    let content = content();
    for ((side, width, seed, archers), sum, first, mid, end) in GODOT_MIXED {
        let (march, settle, mixed) = (true, 0, true); // from the start, as the probe stepped it
        let options = Options { side, width, seed, archers, march, settle, mixed, ..small() };
        let mut settled = settled(&options, &content).expect("the battle sets up");
        let mut told = Vec::new();
        let mut at_mid = String::new();
        for tick in 1..=400 {
            told.push(settled.step(&content));
            if tick == 130 {
                at_mid = sha(&settled.state());
            }
        }
        let first_told = told.iter().position(|&n| n > 0).map_or(-1, |k| k as i64 + 1);
        let ours = (told.iter().sum::<usize>(), first_told, at_mid, sha(&settled.state()));
        assert_eq!(ours, (sum, first, mid.to_string(), end.to_string()), "{options:?}");
    }
}
