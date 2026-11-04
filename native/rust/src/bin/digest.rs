//! The formation digest played in the crate (spec 39 step 8c-xiii), which `bash native/build.sh`
//! builds into `native/bin/breach_digest`: each run of a set played with nothing from Godot, its
//! ticks' records chained with SHA-256 and printed as tools/formation_digest.gd prints them, so
//! the two outputs can be compared line for line (CI's `digests` jobs do).
//!
//!   native/bin/breach_digest [set=standard|clash160|wide] [out=<file of every tick's hash>]
//!       [only=<run label>] [threaded=true|false] [repo=<path>]
//!
//! `out` and `only` are the Godot digest's; `trace` and `snapshot` are still only Godot's.
//! `repo` is the folder holding project.godot (default: the one this is run in or from).

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::digest_runs::{line, runs, tick_line};

const USAGE: &str = "usage: breach_digest [set=standard|clash160|wide] [out=<file>] \
                     [only=<run label>] [threaded=true|false] [repo=<path>]";

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
        eprintln!("breach_digest: unknown argument {unknown} (trace and snapshot are Godot's)");
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let set = args.get("set").map_or("standard", String::as_str);
    let Some(runs) = runs(set) else {
        eprintln!("breach_digest: no set {set}\n{USAGE}");
        return ExitCode::from(2);
    };
    let Some(root) = args.get("repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_digest: no repo found (a folder with project.godot); pass repo=");
        return ExitCode::from(2);
    };
    let content = match Content::load(&root.join("content")) {
        Ok(content) => content,
        Err(why) => {
            eprintln!("breach_digest: the content doesn't load: {why}");
            return ExitCode::from(1);
        }
    };
    let mut out = match args.get("out").map(File::create).transpose() {
        Ok(file) => file.map(BufWriter::new),
        Err(why) => {
            eprintln!("breach_digest: can't write out=: {why}");
            return ExitCode::from(1);
        }
    };
    let threaded = args.get("threaded").is_none_or(|value| value != "false");
    let only = args.get("only").map_or("", String::as_str);
    let mut failed = false;
    for run in runs.iter().filter(|run| only.is_empty() || run.label == only) {
        let mut written = Ok(());
        let (ticks, hash) = run.digest(&content, threaded, &mut |tick, one| {
            if let (Some(out), Ok(())) = (out.as_mut(), &written) {
                written = writeln!(out, "{}", tick_line(&run.label, tick, one));
            }
        });
        match (hash, written) {
            (Some(hash), Ok(())) => println!("{}", line(&run.label, ticks, &hash)),
            (None, _) => {
                eprintln!("breach_digest: {} can't be set up", run.label);
                failed = true;
            }
            (_, Err(why)) => {
                eprintln!("breach_digest: can't write out=: {why}");
                return ExitCode::from(1);
            }
        }
    }
    if let Some(Err(why)) = out.as_mut().map(Write::flush) {
        eprintln!("breach_digest: can't write out=: {why}");
        return ExitCode::from(1);
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

const KNOWN: [&str; 5] = ["set", "out", "only", "threaded", "repo"];

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
