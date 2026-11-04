//! tools/native_bench.gd played in the crate (spec 39 step 8c-xix), which `bash
//! native/build.sh` builds into `native/bin/breach_native_bench`: a mid-fight tick of
//! FormationBench's clash on one thread ("rust") and on the pool ("rust_threads"), with the
//! bodies' parting and the scrum's seek timed inside it, on the crate's own tick with nothing
//! from Godot. A line a run, then min and median over the runs, ms a tick throughout.
//! native_bench.gd's b.sync, b.call and b.write time the Godot boundary, which the crate
//! doesn't cross, and its b.core is the crate's whole parting ("bodies"), so they aren't
//! printed.
//!
//!   native/bin/breach_native_bench [engines=rust,rust_threads] [sizes=160:10,512:32,1024:32]
//!       [runs=3] [settle=10] [fight=10] [seed=1] [repo=<path>]
//!
//! An engine ending "_threads" runs on the pool, any other on one thread, as native_bench.gd
//! reads them. `repo` is the folder holding project.godot (default: the one this is run in or
//! from).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::native_bench::{self, Options};

const USAGE: &str = "usage: breach_native_bench [engines=rust,rust_threads] \
                     [sizes=160:10,512:32,1024:32] [runs=3] [settle=10] [fight=10] [seed=1] \
                     [repo=<path>]";
const KNOWN: [&str; 7] = ["engines", "sizes", "runs", "settle", "fight", "seed", "repo"];

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
        eprintln!("breach_native_bench: unknown argument {unknown}\n{USAGE}");
        return ExitCode::from(2);
    }
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_native_bench: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_native_bench: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let text = |key: &str, default: &str| args.get(key).map_or(default, String::as_str).to_string();
    let count = |key: &str, default: i64| args.get(key).map_or(default, |v| whole(v));
    let engines = text("engines", "rust,rust_threads");
    let sizes = text("sizes", "160:10,512:32,1024:32");
    let runs = count("runs", 3).max(0);
    let options = Options {
        settle: count("settle", 10).max(0) as usize,
        fight: count("fight", 10).max(0) as usize,
        seed: count("seed", 1),
    };
    println!("{}", native_bench::heading());
    for size in sizes.split(',') {
        let (side, width) = size.split_once(':').unwrap_or((size, "0"));
        let at = (whole(side).max(0) as usize, whole(width).max(0) as usize);
        for engine in engines.split(',') {
            let threaded = engine.ends_with("_threads");
            let mut results = Vec::new();
            for run in 1..=runs {
                let Some(mut clash) =
                    native_bench::settled(at.0, at.1, threaded, &options, &content)
                else {
                    eprintln!("breach_native_bench: the clash {size} can't be set up");
                    return ExitCode::from(1);
                };
                results.push(native_bench::measure(&mut clash, options.fight, &content));
                let label = format!("{engine} #{run}");
                println!("{}", native_bench::line(&label, at, &results[results.len() - 1]));
            }
            let min = native_bench::pick(&results, 0.0);
            println!("{}", native_bench::line(&format!("{engine} min"), at, &min));
            let med = native_bench::pick(&results, 0.5);
            println!("{}", native_bench::line(&format!("{engine} med"), at, &med));
        }
    }
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
