//! native_bench.gd's battle held to Godot's: FormationBench.clash brought to its fight and
//! then `fight` ticks on, as native_bench.gd's `_measure` steps it, ends on Godot's tick and
//! digest state (a throwaway Godot probe, spec 39 step 8c-xix); and the columns add up.

use sha2::{Digest, Sha256};

use super::{heading, line, measure, pick, settled, Options};
use crate::content::Content;

/// (side, width), the tick after `_measure`'s ten fight ticks, and the SHA-256 of the
/// digest's state then, as Godot gave them (seed 1, settle 10).
const GODOT: [((usize, usize), i64, &str); 2] = [
    ((64, 8), 110, "45bddae89522c6a74e9a7940b41ecfd4672299c16a214904071c7b7e1571c63d"),
    ((160, 10), 110, "15b95fc717111141213d7cc32dfbfacd8c9527fb55f01bd76fc319af5163fed0"),
];

fn content() -> Content {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    Content::load(&root).expect("the repo's content loads")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn each_measured_clash_ends_where_godot_s_did_on_one_thread_or_the_pool() {
    let content = content();
    let options = Options::default();
    for ((side, width), tick, sha) in GODOT {
        for threaded in [false, true] {
            let mut clash = settled(side, width, threaded, &options, &content).expect("sets up");
            let columns = measure(&mut clash, options.fight, &content);
            assert_eq!(clash.play.tick, tick, "{side}:{width} threaded {threaded}: the tick");
            assert_eq!(hex(&Sha256::digest(clash.state())), sha, "{side}:{width} {threaded}");
            let [all, rest, bodies, seek] = columns;
            assert!(bodies > 0.0 && seek > 0.0, "{columns:?}: the passes are timed");
            assert!(
                (rest + bodies + seek - all).abs() < 1e-9,
                "{columns:?}: rest is the remainder"
            );
        }
    }
}

#[test]
fn the_lines_and_picks_are_native_bench_s() {
    assert_eq!(heading(), "engine             side   w     tick    rest  bodies    seek");
    let line = line("rust #1", (160, 10), &[1.0, 0.5, 0.25, 0.125]);
    assert_eq!(line, "rust #1             160  10     1.00    0.50    0.25    0.12");
    let runs = [[3.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [2.0, 0.0, 0.0, 0.0]];
    assert_eq!((pick(&runs, 0.0)[0], pick(&runs, 0.5)[0]), (1.0, 2.0));
}
