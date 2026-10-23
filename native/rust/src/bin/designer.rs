//! The map designer's local server (specs/17-local-designer-app.md, Decision 30; in Rust since
//! spec 39 step 2c), which `bash native/build.sh` builds into `native/bin/breach_designer`:
//!
//!   native/bin/breach_designer [--port 8765] [--godot <path>] [--repo <path>] [--no-browser]
//!
//! It serves the designer from `tools/designer/` on 127.0.0.1 and saves maps and libraries
//! into the repo (`designer::repo`). Godot (--godot, else GODOT_BIN, else
//! tools/designer/local_config.json's "godot") is only needed to open the map viewer.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use breach_native::designer::http::Server;
use breach_native::designer::repo::{find_godot, launch_detached, Repo};

const USAGE: &str =
    "usage: breach_designer [--port 8765] [--godot <path>] [--repo <path>] [--no-browser]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        let at = args.iter().position(|arg| arg == flag)?;
        args.get(at + 1).cloned()
    };
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let Some(root) = value("--repo").map(PathBuf::from).or_else(find_repo) else {
        eprintln!("breach_designer: no repo found (a folder with project.godot); pass --repo");
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let port: u16 = value("--port").and_then(|port| port.parse().ok()).unwrap_or(8765);
    let designer = root.join("tools/designer");
    let env = std::env::var("GODOT_BIN").ok();
    let config = designer.join("local_config.json");
    let godot = find_godot(value("--godot").as_deref(), env.as_deref(), &config);
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("breach_designer: can't listen on 127.0.0.1:{port}: {why}");
            return ExitCode::from(1);
        }
    };
    let url = format!("http://127.0.0.1:{port}/");
    println!("Lane Tile Designer: {url}  (repo {})", root.display());
    match &godot {
        Some(godot) => println!("Godot: {godot} - View in Godot opens the map viewer"),
        None => println!(
            "Godot: not configured - maps save, but View in Godot can't open the viewer.\n       \
             Pass --godot <path>, set GODOT_BIN, or add {{\"godot\": \"<path>\"}} to\n       \
             tools/designer/local_config.json (gitignored)."
        ),
    }
    println!("Ctrl+C to stop.");
    if !args.iter().any(|arg| arg == "--no-browser") {
        open_browser(&url);
    }
    let server = Server::new(Repo::new(root, godot, Box::new(launch_detached)), designer);
    Arc::new(server).serve(listener);
    ExitCode::SUCCESS
}

/// The repo: the first folder holding project.godot, from here up, else from this program
/// (native/bin/) up.
fn find_repo() -> Option<PathBuf> {
    let here = std::env::current_dir().ok();
    let program = std::env::current_exe().ok();
    let starts = [here.as_deref(), program.as_deref().and_then(Path::parent)];
    let found = starts.into_iter().flatten().find_map(|start| {
        start.ancestors().find(|dir| dir.join("project.godot").is_file()).map(Path::to_path_buf)
    });
    found
}

fn open_browser(url: &str) {
    let command: &[&str] = if cfg!(windows) {
        &["cmd", "/C", "start", ""]
    } else if cfg!(target_os = "macos") {
        &["open"]
    } else {
        &["xdg-open"]
    };
    let mut all: Vec<String> = command.iter().map(|part| part.to_string()).collect();
    all.push(url.to_string());
    let _ = launch_detached(&all);
}
