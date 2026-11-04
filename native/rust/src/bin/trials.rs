//! BattleTrials played in the crate (spec 39 step 8c-xiv), which `bash native/build.sh` builds
//! into `native/bin/breach_trials`: each scenario fought once per battle seed with nothing from
//! Godot, and its wins and losses printed as tools/formation_trials.gd prints them, so the two
//! outputs can be compared line for line (CI's `trials` job does).
//!
//!   native/bin/breach_trials [scenario=all|<name>] [runs=50] [rolls=off] [captain]
//!       [first_seed=1] [swap] [repeats=20] [threaded=true|false] [repo=<path>]
//!
//! The scenarios are BattleTrials': mirror_headon, mirror_flank, field_a, field_b,
//! field_b_waits and field_together; and `fatigue` (FatigueTrials, step 8c-xv): how far a line
//! pursues after 3, 30, 60 and 90 s of contact; and `paths` (PathTrials and SightTrials, step
//! 8c-xvi): ways over the field for four walkers, drawn and timed, each search the mean of
//! `repeats` (default 20), and a grem planning on what it sees feeling along a wall to its gap.
//! `repo` is the folder holding project.godot (default: the one this is run in or from).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::trials::paths::{self, wall};
use breach_native::trials::{self, columns, fatigue, heading, row, summary, Options, SCENARIOS};

const USAGE: &str = "usage: breach_trials [scenario=all|<name>] [runs=50] [rolls=off] [captain] \
                     [first_seed=1] [swap] [repeats=20] [threaded=true|false] [repo=<path>]";
const KNOWN: [&str; 9] =
    ["scenario", "runs", "rolls", "captain", "first_seed", "swap", "repeats", "threaded", "repo"];

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
        eprintln!("breach_trials: unknown argument {unknown}\n{USAGE}");
        return ExitCode::from(2);
    }
    let scenario = args.get("scenario").map_or("all", String::as_str);
    let names: Vec<&str> = if scenario == "all" { SCENARIOS.to_vec() } else { vec![scenario] };
    let (tires, ways) = (scenario == "fatigue", scenario == "paths");
    if let Some(name) = names.iter().find(|name| !tires && !ways && !SCENARIOS.contains(name)) {
        eprintln!("breach_trials: no scenario {name}\n{USAGE}");
        return ExitCode::from(2);
    }
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_trials: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_trials: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let runs_given = args.get("runs").map_or("50", String::as_str);
    let threaded = args.get("threaded").is_none_or(|value| value != "false");
    let first_seed = whole(args.get("first_seed").map_or("1", String::as_str));
    if ways {
        return find_ways(whole(args.get("repeats").map_or("20", String::as_str)), &content);
    }
    if tires {
        return tire(whole(runs_given), first_seed, threaded, &content);
    }
    let options = Options {
        rolls: args.get("rolls").is_none_or(|value| value != "off"),
        captain: args.contains_key("captain"),
        first_seed,
        swap: args.contains_key("swap"),
        threaded,
    };
    println!("{}", heading(runs_given, &options));
    println!("{}", columns());
    for name in names {
        let Some(results) = trials::run(name, whole(runs_given), &options, &content) else {
            eprintln!("breach_trials: {name} can't be set up");
            return ExitCode::from(1);
        };
        println!("{}", row(name, &summary(&results)));
    }
    ExitCode::SUCCESS
}

/// formation_trials.gd's `_fatigue`: a line a contact, each over `runs` battles.
fn tire(runs: i64, first_seed: i64, threaded: bool, content: &Content) -> ExitCode {
    println!("{}", fatigue::columns());
    for seconds in fatigue::CONTACTS {
        let Some(result) = fatigue::run(seconds, runs, first_seed, threaded, content) else {
            eprintln!("breach_trials: fatigue can't be set up");
            return ExitCode::from(1);
        };
        println!("{}", fatigue::row(seconds, &result));
    }
    ExitCode::SUCCESS
}

/// formation_trials.gd's `paths`: PathTrials' lines, then SightTrials'.
fn find_ways(repeats: i64, content: &Content) -> ExitCode {
    let (Some(rows), Some(ground)) = (paths::run(repeats, content), paths::ground(content)) else {
        eprintln!("breach_trials: paths can't be set up");
        return ExitCode::from(1);
    };
    println!("{}", paths::report(&rows, &ground).join("\n"));
    println!("{}", wall::report(&wall::feel_along(content), content).join("\n"));
    ExitCode::SUCCESS
}

/// A number as GDScript's `int(String)` reads it here: its digits, or 0.
fn whole(text: &str) -> i64 {
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
