//! tools/tick_phases.gd played in the crate (spec 39 step 8c-xviii), which `bash
//! native/build.sh` builds into `native/bin/breach_tick_phases`: where a mid-fight tick's time
//! goes, pass by pass, timed on the crate's own tick with nothing from Godot, printed as
//! tick_phases.gd prints it, so `bench_compare.py` (the `bench` workflow) reads it.
//!
//!   native/bin/breach_tick_phases [side=1024] [width=32] [settle=10] [fight=5] [seed=1]
//!       [stage=fight|march] [retreat=<squad id>] [squads=1] [archers=0]
//!       [threaded=true|false] [repo=<path>]
//!
//! tick_phases.gd's `copies` counts column copies across the Godot boundary, which the crate's
//! tick doesn't cross: it exits 2. `repo` is the folder holding project.godot (default: the
//! one this is run in or from).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::phases::{self, Options, WARNING};

const USAGE: &str = "usage: breach_tick_phases [side=1024] [width=32] [settle=10] [fight=5] \
                     [seed=1] [stage=fight|march] [retreat=<squad id>] [squads=1] [archers=0] \
                     [threaded=true|false] [repo=<path>]";
const KNOWN: [&str; 11] = [
    "side", "width", "settle", "fight", "seed", "stage", "retreat", "squads", "archers",
    "threaded", "repo",
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
    if args.contains_key("copies") {
        eprintln!("breach_tick_phases: copies counts Godot's column copies; the crate makes none");
        return ExitCode::from(2);
    }
    if let Some(unknown) = args.keys().find(|key| !KNOWN.contains(&key.as_str())) {
        eprintln!("breach_tick_phases: unknown argument {unknown}\n{USAGE}");
        return ExitCode::from(2);
    }
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_tick_phases: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_tick_phases: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let options = options(&args);
    let (Some(mut timed), Some(mut plain)) =
        (phases::settled(&options, &content), phases::settled(&options, &content))
    else {
        eprintln!("breach_tick_phases: the battle can't be set up");
        return ExitCode::from(1);
    };
    let spent = phases::timed(&mut timed, options.fight, &content);
    for _ in 0..options.fight {
        plain.step(&content);
    }
    println!("{}", phases::machine());
    println!("{}", phases::lines(&options, &spent).join("\n"));
    if timed.state() != plain.state() {
        println!("{WARNING}");
    }
    ExitCode::SUCCESS
}

/// The options given, each as tick_phases.gd reads it, the rest its defaults.
fn options(args: &HashMap<String, String>) -> Options {
    let given = Options::default();
    let count =
        |key: &str, default: usize| args.get(key).map_or(default, |v| whole(v).max(0) as usize);
    Options {
        side: count("side", given.side),
        width: count("width", given.width),
        settle: count("settle", given.settle),
        fight: count("fight", given.fight),
        seed: args.get("seed").map_or(given.seed, |v| whole(v)),
        march: args.get("stage").is_some_and(|stage| stage == "march"),
        retreat: args.get("retreat").map(|v| whole(v)),
        squads: count("squads", given.squads),
        archers: count("archers", given.archers),
        threaded: args.get("threaded").is_none_or(|value| value != "false"),
    }
}

/// A number as GDScript's `int(String)` reads it here: its digits, or 0.
fn whole(text: &str) -> i64 {
    text.trim().parse::<i64>().unwrap_or(0)
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
