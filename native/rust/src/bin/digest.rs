//! The formation digest played in the crate (spec 39 step 8c-xiii), which `bash native/build.sh`
//! builds into `native/bin/breach_digest`: each run of a set played with nothing from Godot, its
//! ticks' records chained with SHA-256 and printed as tools/formation_digest.gd prints them, so
//! the two outputs can be compared line for line (CI's `digests` jobs do).
//!
//!   native/bin/breach_digest [set=standard|clash160|wide] [out=<file of every tick's hash>]
//!       [only=<run label>] [trace=<file>] [from=<tick>] [to=<tick>] [threaded=true|false|always]
//!       [repo=<path>]
//!
//!   native/bin/breach_digest cuts=<dir> [repo=<path>]
//!
//! `only` plays that one run (its label as printed, e.g. "mirror_headon s1 rtrue"). `trace`
//! writes each tick from `from` to `to` (default every tick) as a line of JSON naming every
//! field (spec 37, `digest::trace`); the hashes are the same with it or without. The Godot
//! digest, which this was checked against, retired in spec 39 step 9d-iii, and its
//! `snapshot=` with it: `cuts` took its place (step 8d).
//! `cuts` writes the snapshot tests' cuts (spec 39 step 8d, `breach_native::cuts::CUTS`) to
//! `<dir>` instead: each one's state (`<name>.state`) and its run, tick, squads listed and
//! each lap's print (`<name>.txt`), and a field log's own state (`<name>.field`). Its tests
//! read them from `native/rust/cuts`.
//! `repo` is the folder holding project.godot (default: the one this is run in or from).

use breach_native::hashing::{HashMap, HashMapExt};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use breach_native::content::Content;
use breach_native::cuts;
use breach_native::digest_runs::{line, runs, tick_line};
use breach_native::digest_trace::{self, traced};
use breach_native::threads::Threads;

const USAGE: &str = "usage: breach_digest [set=standard|clash160|wide] [out=<file>] \
                     [only=<run label>] [trace=<file>] [from=<tick>] [to=<tick>] \
                     [threaded=true|false|always] [repo=<path>]\n       \
                     breach_digest cuts=<dir> [repo=<path>]";

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
        eprintln!("breach_digest: unknown argument {unknown}");
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
    if let Some(dir) = args.get("cuts") {
        return write_cuts(Path::new(dir), &content);
    }
    let (Ok(mut out), Ok(mut trace)) = (create(&args, "out"), create(&args, "trace")) else {
        return ExitCode::from(1);
    };
    let (Ok(from), Ok(to)) = (tick_of(&args, "from", 1), tick_of(&args, "to", -1)) else {
        eprintln!("breach_digest: from= and to= are ticks\n{USAGE}");
        return ExitCode::from(2);
    };
    let Some(threaded) = Threads::parse(args.get("threaded").map(String::as_str)) else {
        eprintln!("breach_digest: threaded= is true, false or always\n{USAGE}");
        return ExitCode::from(2);
    };
    let only = args.get("only").map_or("", String::as_str);
    let mut failed = false;
    for run in runs.iter().filter(|run| only.is_empty() || run.label == only) {
        let mut written = Ok(());
        let (ticks, hash) = run.digest(&content, threaded, &mut |tick, one, told| {
            if let (Some(out), Ok(())) = (out.as_mut(), &written) {
                written = writeln!(out, "{}", tick_line(&run.label, tick, one));
            }
            if let (Some(trace), Ok(()), true) = (trace.as_mut(), &written, traced(tick, from, to))
            {
                let factions = told.factions();
                let traced =
                    digest_trace::line(&run.label, tick, &told.events, &told.state, &factions);
                written = writeln!(trace, "{traced}");
            }
        });
        match (hash, written) {
            (Some(hash), Ok(())) => println!("{}", line(&run.label, ticks, &hash)),
            (None, _) => {
                eprintln!("breach_digest: {} can't be set up", run.label);
                failed = true;
            }
            (_, Err(why)) => {
                eprintln!("breach_digest: can't write out= or trace=: {why}");
                return ExitCode::from(1);
            }
        }
    }
    for file in [out.as_mut(), trace.as_mut()].into_iter().flatten() {
        if let Err(why) = file.flush() {
            eprintln!("breach_digest: can't write out= or trace=: {why}");
            return ExitCode::from(1);
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

const KNOWN: [&str; 9] = ["set", "out", "only", "trace", "from", "to", "threaded", "repo", "cuts"];

/// The file `key=` names, created (None if not given); Err, told, if it can't be.
fn create(args: &HashMap<String, String>, key: &str) -> Result<Option<BufWriter<File>>, ()> {
    match args.get(key).map(File::create).transpose() {
        Ok(file) => Ok(file.map(BufWriter::new)),
        Err(why) => {
            eprintln!("breach_digest: can't write {key}=: {why}");
            Err(())
        }
    }
}

/// The tick `key=` gives, or `default`.
fn tick_of(args: &HashMap<String, String>, key: &str, default: i64) -> Result<i64, ()> {
    args.get(key).map_or(Ok(default), |value| value.parse().map_err(|_| ()))
}

/// Writes every cut of `CUTS` to `dir`: its state, its text and a field log's own state.
fn write_cuts(dir: &Path, content: &Content) -> ExitCode {
    for (label, tick) in cuts::CUTS {
        let Some((cut, prints)) =
            cuts::run_of(label).and_then(|run| cuts::cut(&run, tick, content))
        else {
            eprintln!("breach_digest: {label} doesn't reach tick {tick}");
            return ExitCode::from(1);
        };
        let name = cuts::file_name(label, tick);
        let wrote = std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(dir.join(format!("{name}.state")), &cut.bytes))
            .and_then(|()| {
                std::fs::write(dir.join(format!("{name}.txt")), cuts::written(&cut, &prints))
            })
            .and_then(|()| match cut.carried.is_empty() {
                true => Ok(()),
                false => std::fs::write(dir.join(format!("{name}.field")), &cut.carried),
            });
        if let Err(why) = wrote {
            eprintln!("breach_digest: can't write {name}: {why}");
            return ExitCode::from(1);
        }
        println!("{name}");
    }
    ExitCode::SUCCESS
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
