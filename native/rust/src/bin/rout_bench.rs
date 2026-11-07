//! tools/rout_bench.gd played in the crate (spec 39 step 8c-xix), which `bash native/build.sh`
//! builds into `native/bin/breach_rout_bench`: how long a tick of routs takes with many
//! friendly formations, the rout's pass alone timed on the crate's own tick with nothing from
//! Godot, printed as rout_bench.gd prints it.
//!
//!   native/bin/breach_rout_bench [side=1024] [squads=16] [width=16] [ticks=20] [repo=<path>]
//!
//! `repo` is the folder holding project.godot (default: the one this is run in or from).

use breach_native::hashing::{HashMap, HashMapExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use breach_native::content::Content;
use breach_native::rout_bench::{self, Options};

const USAGE: &str =
    "usage: breach_rout_bench [side=1024] [squads=16] [width=16] [ticks=20] [repo=<path>]";
const KNOWN: [&str; 5] = ["side", "squads", "width", "ticks", "repo"];

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
        eprintln!("breach_rout_bench: unknown argument {unknown}\n{USAGE}");
        return ExitCode::from(2);
    }
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_rout_bench: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_rout_bench: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let given = Options::default();
    let count =
        |key: &str, default: usize| args.get(key).map_or(default, |v| whole(v).max(0) as usize);
    let options = Options {
        side: count("side", given.side),
        squads: count("squads", given.squads),
        width: count("width", given.width),
        ticks: count("ticks", given.ticks),
    };
    let Some(mut routs) = rout_bench::set_up(&options, &content) else {
        eprintln!("breach_rout_bench: the battle can't be set up");
        return ExitCode::from(1);
    };
    let mut events = 0;
    let began = Instant::now();
    for tick in 1..=options.ticks as i64 {
        events += routs.step(tick);
    }
    let ms = began.elapsed().as_secs_f64() * 1000.0 / options.ticks.max(1) as f64;
    println!("{}", rout_bench::line(&options, ms, events, routs.routing()));
    ExitCode::SUCCESS
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
