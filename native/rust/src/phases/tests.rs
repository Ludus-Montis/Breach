use super::*;
use crate::content::tests::repo_content;

pub(crate) fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

fn small() -> Options {
    Options { side: 64, width: 8, settle: 3, fight: 2, ..Options::default() }
}

/// The phases a tick laps, in the order first lapped: the crate's passes, as tick.rs and its
/// five passes run them, each named by its last part (what a watcher is shown).
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
    "scrum: facing",
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
    "rout: listing",
    "carry",
    "carry: strays",
    "groups (form up, split)",
    "stamina",
    "bodies: commit",
    "shuffle (re-form)",
    "units walk to places",
    "store settles",
];

/// The laps a tick times, in the order first lapped: each phase's parts (spec 41 step 1:
/// the splits, then the lap that ends the phase), as tick.rs and its five passes run them.
pub(crate) const PARTS: &[&str] = &[
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
    "scrum: seek",
    "scrum: walk",
    "scrum: facing",
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
    "rout: listing",
    "carry",
    "carry: strays",
    "groups (form up, split)",
    "stamina",
    "bodies: sync",
    "bodies: pair search",
    "bodies: pushes",
    "bodies: sums and applying",
    "bodies: commit",
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
    assert_eq!(names, PARTS);
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
/// many events each of those six told of. Since spec 42 step 2 (Decision 161: seek's claims
/// settled in rounds) these are the crate's own, its digests the baseline.
#[allow(clippy::type_complexity)]
const GODOT_S: [((usize, usize, bool, Option<i64>, i64, usize), i64, [usize; 6]); 5] = [
    ((3, 1, false, None, 1, 0), 99, [4, 0, 0, 0, 0, 0]),
    ((3, 1, false, Some(1), 2, 0), 99, [7, 0, 0, 0, 0, 0]),
    ((10, 2, false, None, 1, 0), 106, [1, 1, 26, 14, 1, 8]),
    ((3, 1, false, None, 1, 2), 99, [4, 0, 0, 0, 0, 0]),
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
/// DigestState.state at tick 130 (the fight) and at tick 400 (long after it ended). Since
/// spec 42 step 2 (Decision 161) the crate's own.
type Mixed = ((usize, usize, i64, usize), usize, i64, &'static str, &'static str);

const GODOT_MIXED: [Mixed; 3] = [
    (
        (64, 8, 1, 0),
        449,
        108,
        "52b3a3fe751b99fda14850fb372bfd0c7415a4a7a23cb82edca29e90c6c2d837",
        "1ebb08c448521a2499942dcda0d3ee902425384d87956c68a0b6c3abefbf98b8",
    ),
    (
        (64, 8, 2, 0),
        474,
        108,
        "2f9bfdaff4af66c920a0d3f756d2f39cdb4b51ccbe9ccfd85bc476027bb1eb1d",
        "ac0b2224bc5c54aee058c7caaa129a36dcf8d62d37a3c7fbc4d51d4ac87abfea",
    ),
    (
        (96, 12, 1, 4),
        549,
        108,
        "4a97bc12889f9483c52b6ec95a4899dc491896bd11e06f048bf55671dccfd950",
        "9e41295bd97d811270a1c2849318a6653d436a982d77768dc0e753e304107e53",
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
