//! tools/scaling_check.gd played in the crate (spec 39 step 8c-xvii), which `bash
//! native/build.sh` builds into `native/bin/breach_scaling`: how a tick's cost grows as the
//! battle doubles, timed on the crate's own tick with nothing from Godot, printed as
//! scaling_check.gd prints it. Exits 1 when a doubling costs more than `limit` times as much.
//!
//!   native/bin/breach_scaling [sizes=128,256,512] [ranks=8] [ticks=6] [runs=3] [limit=2.5]
//!       [stacks=1,2,4] [stack=128] [settle=20] [stages=march,fight,stacks]
//!       [threaded=true|false|always] [repo=<path>]
//!
//! `repo` is the folder holding project.godot (default: the one this is run in or from).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::scaling::{self, Options, FAILED, HEADING, STAGES};
use breach_native::threads::Threads;

const USAGE: &str = "usage: breach_scaling [sizes=128,256,512] [ranks=8] [ticks=6] [runs=3] \
                     [limit=2.5] [stacks=1,2,4] [stack=128] [settle=20] \
                     [stages=march,fight,stacks] [threaded=true|false|always] [repo=<path>]";
const KNOWN: [&str; 11] = [
    "sizes", "ranks", "ticks", "runs", "limit", "stacks", "stack", "settle", "stages", "threaded",
    "repo",
];

fn main() -> ExitCode {
    let mut args = HashMap::new();
    for arg in std::env::args().skip(1) {
        let (key, value) = arg.split_once('=').unwrap_or((&arg, "true"));
        args.insert(key.to_string(), value.to_string());
    }
    if args.contains_key("--help") || args.contains_key("-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if let Some(unknown) = args.keys().find(|key| !KNOWN.contains(&key.as_str())) {
        eprintln!("breach_scaling: unknown argument {unknown}\n{USAGE}");
        return ExitCode::from(2);
    }
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_scaling: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_scaling: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let options = options(&args);
    let stages = args.get("stages").map_or(STAGES.join(","), String::clone);
    let chosen: Vec<&str> = stages.split(',').collect();
    let mut failures = 0;
    println!("{HEADING}");
    for stage in STAGES.into_iter().filter(|stage| chosen.contains(stage)) {
        let sizes = scaling::sizes(stage, &options);
        let mut spent = Vec::new();
        for &size in sizes {
            let Some(ms) = scaling::least(stage, size, &options, &content) else {
                eprintln!("breach_scaling: {stage} can't be set up");
                return ExitCode::from(1);
            };
            spent.push(ms);
        }
        let (lines, failed) = scaling::lines(stage, sizes, &spent, options.limit);
        println!("{}", lines.join("\n"));
        failures += failed;
    }
    if failures > 0 {
        println!("{FAILED}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// The options given, each as scaling_check.gd reads it, the rest its defaults.
fn options(args: &HashMap<String, String>) -> Options {
    let given = Options::default();
    let count = |key: &str, default: usize| args.get(key).map_or(default, |v| whole(v));
    let counts = |key: &str, default: &[usize]| {
        args.get(key).map_or(default.to_vec(), |v| v.split(',').map(whole).collect())
    };
    Options {
        sizes: counts("sizes", &given.sizes),
        ranks: count("ranks", given.ranks),
        ticks: count("ticks", given.ticks),
        runs: count("runs", given.runs),
        limit: args.get("limit").map_or(given.limit, |v| v.trim().parse().unwrap_or(0.0)),
        stacks: counts("stacks", &given.stacks),
        stack: count("stack", given.stack),
        settle: count("settle", given.settle),
        threaded: Threads::parse(args.get("threaded").map(String::as_str)).unwrap_or(Threads::Pool),
    }
}

/// A count as GDScript's `int(String)` reads it here: its digits, or 0.
fn whole(text: &str) -> usize {
    text.trim().parse().unwrap_or(0)
}

/// The repo this is run in or from: the nearest folder above holding project.godot.
fn find_repo() -> Option<PathBuf> {
    let here = std::env::current_dir().ok();
    let program = std::env::current_exe().ok();
    let starts = [here.as_deref(), program.as_deref().and_then(Path::parent)];
    let found = starts.into_iter().flatten().find_map(|start| {
        start.ancestors().find(|dir| dir.join("project.godot").is_file()).map(Path::to_path_buf)
    });
    found
}
