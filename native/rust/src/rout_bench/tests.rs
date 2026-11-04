//! rout_bench.gd's battle held to Godot's: what FormationRout.step told of each tick, how many
//! formations ended routing, and the digest's state at the end, from a throwaway Godot probe
//! of rout_bench.gd's `_field` and loop (spec 39 step 8c-xix).

use sha2::{Digest, Sha256};

use super::{line, set_up, Options};
use crate::content::Content;

/// (side, squads, width, ticks), each tick's events, the formations routing at the end, and
/// the SHA-256 of the digest's state then, as Godot gave them.
/// A run's options as (side, squads, width, ticks), then what Godot told of.
type Run = ((usize, usize, usize, usize), &'static [usize], usize, &'static str);

const GODOT: [Run; 3] = [
    (
        (128, 8, 8, 20),
        &[8, 0, 0, 0, 0, 0, 8, 16, 8, 0, 0, 0, 0, 9, 16, 8, 0, 0, 0, 0],
        4,
        "7b2095d6d2839e3a592925dd2d407c9fd0c6eccdf696cee02190058c9591e0f9",
    ),
    (
        (256, 4, 16, 20),
        &[2, 0, 0, 0, 0, 0, 17, 32, 32, 32, 16, 0, 0, 0, 0, 0, 17, 32, 32, 32],
        2,
        "19de93a1f9b142122ead803af80764138901e6a8b0b5525b15837168bcb8fc7f",
    ),
    (
        (64, 16, 4, 30),
        &[
            15, 0, 0, 0, 0, 0, 4, 4, 0, 0, 0, 0, 4, 5, 3, 0, 0, 0, 0, 0, 12, 13, 3, 0, 0, 0, 0, 0,
            17, 18,
        ],
        11,
        "4d945666d44c8347acbfb016729e625e96b8596b85f37b0926b3e89e5c7a6baf",
    ),
];

fn content() -> Content {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    Content::load(&root).expect("the repo's content loads")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn each_tick_of_routs_tells_of_what_godot_s_did_and_ends_as_it_did() {
    let content = content();
    for ((side, squads, width, ticks), told, routing, sha) in GODOT {
        let options = Options { side, squads, width, ticks };
        let mut routs = set_up(&options, &content).expect("the battle sets up");
        let ours: Vec<usize> = (1..=ticks as i64).map(|tick| routs.step(tick)).collect();
        assert_eq!(ours, told, "{options:?}: events each tick");
        assert_eq!(routs.routing(), routing, "{options:?}: routing at the end");
        assert_eq!(hex(&Sha256::digest(routs.state())), sha, "{options:?}: the state at the end");
    }
}

#[test]
fn the_line_is_rout_bench_s() {
    let options = Options::default();
    assert_eq!(
        line(&options, 1.234, 56, 7),
        "rout: 1024 a side in 16 formations, 16 wide: 1.23 ms a tick (56 events, 7 routing)"
    );
}
