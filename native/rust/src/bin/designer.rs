//! What the designer's server (`tools/designer/designer_repo.py`) runs to read and write the
//! simulation's content in the crate's schema (spec 39 step 2c). Godot isn't needed.
//!
//!   designer map <export.json> <map.json> <content dir>
//!       reads a saved designer map into the schema, checks it against the content, writes
//!       it; prints `warning: ...` and `error: ...` lines, then `wrote <map.json>`, and exits
//!       1 (writing nothing) on an error.
//!   designer terrain-read <content dir>
//!       prints the terrain library in the designer's shape.
//!   designer terrain-write <content dir> <maps_src dir>
//!       reads the designer's terrain library on stdin and writes it in the schema, unless it
//!       drops a terrain or feature a saved map uses, or doesn't fit the schema; prints
//!       `error: ...` lines then, and exits 1.

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use breach_native::content::{file_text, load_file, LoadError, Map, TerrainLibrary};
use breach_native::designer::{map_from_export, terrain};
use serde_json::Value;

const TERRAIN_LIBRARY: &str = "terrain/terrain_library.json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["map", export, out, content] => map(Path::new(export), Path::new(out), Path::new(content)),
        ["terrain-read", content] => terrain_read(Path::new(content)),
        ["terrain-write", content, maps_src] => {
            terrain_write(Path::new(content), Path::new(maps_src))
        }
        _ => {
            eprintln!(
                "usage: designer map <export.json> <map.json> <content dir> | terrain-read \
                 <content dir> | terrain-write <content dir> <maps_src dir>"
            );
            return ExitCode::from(2);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(errors) => {
            for error in errors {
                eprintln!("error: {error}");
            }
            ExitCode::from(1)
        }
    }
}

fn map(export_path: &Path, out: &Path, content: &Path) -> Result<(), Vec<String>> {
    let export: Value = read_json(export_path)?;
    let library: TerrainLibrary = load_file(&content.join(TERRAIN_LIBRARY)).map_err(one)?;
    let read = map_from_export(&export, &library);
    for warning in &read.warnings {
        println!("warning: {warning}");
    }
    if !read.errors.is_empty() {
        return Err(read.errors);
    }
    let text = file_text(&read.map);
    // The file must read back as the crate loads a map: the schema, and every reference.
    let back: Map = serde_json::from_str(&text).map_err(|error| vec![error.to_string()])?;
    if let Some((field, id)) = back.missing_reference(&library) {
        return Err(vec![format!("{field}: no {id} here")]);
    }
    let broken = back.validate(&library);
    if !broken.is_empty() {
        return Err(broken);
    }
    write(out, &text)?;
    println!("wrote {}", out.display());
    Ok(())
}

fn terrain_read(content: &Path) -> Result<(), Vec<String>> {
    let library: TerrainLibrary = load_file(&content.join(TERRAIN_LIBRARY)).map_err(one)?;
    println!("{}", file_text(&terrain::from_schema(&library)).trim_end());
    Ok(())
}

fn terrain_write(content: &Path, maps_src: &Path) -> Result<(), Vec<String>> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).map_err(|error| vec![error.to_string()])?;
    let designer: Value = serde_json::from_str(&text).map_err(|error| vec![error.to_string()])?;
    let library = terrain::to_schema(&designer).map_err(|error| vec![error])?;
    if let Some((field, id)) = library.missing_material() {
        return Err(vec![format!("{field}: no material {id}")]);
    }
    let problems = terrain::in_use_problems(&library, maps_src);
    if !problems.is_empty() {
        return Err(problems);
    }
    let out = content.join(TERRAIN_LIBRARY);
    write(&out, &file_text(&library))?;
    println!("wrote {}", out.display());
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| vec![format!("{}: {error}", path.display())])?;
    serde_json::from_str(&text).map_err(|error| vec![format!("{}: {error}", path.display())])
}

/// Writes `text` to `path` unless it already holds it.
fn write(path: &Path, text: &str) -> Result<(), Vec<String>> {
    if std::fs::read_to_string(path).is_ok_and(|held| held == text) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| vec![error.to_string()])?;
    }
    std::fs::write(path, text).map_err(|error| vec![format!("{}: {error}", path.display())])
}

fn one(error: LoadError) -> Vec<String> {
    vec![error.to_string()]
}
