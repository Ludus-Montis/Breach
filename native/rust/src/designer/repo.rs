//! What the designer's server does to the repo (specs/17-local-designer-app.md, Decision 30;
//! in Rust since spec 39 step 2c). It touches only these, by whitelisted name, so a request
//! can never reach an arbitrary path:
//!
//! - `content/maps_src/<name>.designer.json`: the designer's saved map (its export), which it
//!   reopens;
//! - `content/maps/<name>.json`: the map in the crate's schema, which the game loads, written
//!   from the export when the map is saved;
//! - `content/designer/<library>.json`: the designer's shared libraries;
//! - `content/terrain/terrain_library.json`: the terrain library, in the crate's schema.
//!
//! Godot is only needed to open the map viewer.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::terrain::{self, MAP_SUFFIX};
use super::{map_from_export, FORMAT};
use crate::content::{file_text, load_file, TerrainLibrary};

/// The designer's libraries, each `content/designer/<name>.json` but the terrain library.
pub const LIBRARIES: [&str; 7] = [
    "factions",
    "default_relations",
    "terrain",
    "emplacements",
    "room_features",
    "room_prefabs",
    "structure_prefabs",
];
const VIEWER_SCENE: &str = "res://presentation/map_viewer.tscn";
const TERRAIN_LIBRARY: &str = "content/terrain/terrain_library.json";

/// Opens a program (the Godot map viewer) without waiting for it.
pub type Launcher = Box<dyn Fn(&[String]) -> std::io::Result<()> + Send + Sync>;

/// The designer's view of one repo checkout.
pub struct Repo {
    pub root: PathBuf,
    pub godot: Option<String>,
    launcher: Launcher,
}

/// Letters, digits, `_` and `-`, 1 to 64 of them: a name that can't leave its folder.
pub fn is_valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Writes `value` as pretty JSON with LF endings and a trailing newline; false (and the
/// file untouched) if it already holds that.
pub fn write_json(path: &Path, value: &Value) -> Result<bool, String> {
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())? + "\n";
    write_text(path, &text)
}

fn write_text(path: &Path, text: &str) -> Result<bool, String> {
    if fs::read_to_string(path).is_ok_and(|held| held == text) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(true)
}

/// The JSON in `path`; none if there is no such file.
pub fn read_json(path: &Path) -> Result<Option<Value>, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    serde_json::from_str(&text).map(Some).map_err(|error| format!("{}: {error}", path.display()))
}

/// Git Bash exports paths like `/f/Code/godot.exe`; Windows needs `F:/Code/godot.exe`.
pub fn to_native_path(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let drive =
        bytes.len() > 2 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b'/';
    if cfg!(windows) && drive {
        format!("{}:/{}", (bytes[1] as char).to_ascii_uppercase(), &raw[3..])
    } else {
        raw.to_string()
    }
}

/// The first of: `--godot`, `GODOT_BIN`, `local_config.json`'s "godot". None if none is set.
pub fn find_godot(cli: Option<&str>, env: Option<&str>, config: &Path) -> Option<String> {
    let configured = read_json(config).ok().flatten();
    let from_config = configured.as_ref().and_then(|c| c.get("godot")).and_then(Value::as_str);
    let found = [cli, env, from_config].into_iter().flatten().find(|found| !found.is_empty());
    found.map(to_native_path)
}

/// The result of writing content from the designer's: whether it ran, whether it was
/// written, and the warnings and errors, as the page shows them.
fn outcome(warnings: Vec<String>, errors: Vec<String>, output: Option<String>) -> Value {
    json!({
        "ran": true,
        "ok": errors.is_empty(),
        "warnings": warnings,
        "errors": errors,
        "output_path": if errors.is_empty() { output } else { None },
    })
}

impl Repo {
    pub fn new(root: PathBuf, godot: Option<String>, launcher: Launcher) -> Repo {
        Repo { root, godot, launcher }
    }

    fn content(&self) -> PathBuf {
        self.root.join("content")
    }

    fn maps_src(&self) -> PathBuf {
        self.content().join("maps_src")
    }

    fn map_out(&self, name: &str) -> PathBuf {
        self.content().join("maps").join(format!("{name}.json"))
    }

    /// Each saved map: {name, modified (seconds), imported (its map file is there)}.
    pub fn list_maps(&self) -> Value {
        let mut paths: Vec<PathBuf> = fs::read_dir(self.maps_src())
            .map(|entries| entries.filter_map(|entry| entry.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        paths.sort();
        let maps = paths.iter().filter_map(|path| {
            let file = path.file_name()?.to_string_lossy().into_owned();
            let name = file.strip_suffix(MAP_SUFFIX).filter(|name| is_valid_name(name))?;
            let modified = fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0.0, |since| since.as_secs_f64());
            Some(json!({"name": name, "modified": modified, "imported": self.map_out(name).is_file()}))
        });
        Value::Array(maps.collect())
    }

    /// The saved export of map `name`; none if it isn't saved.
    pub fn read_map(&self, name: &str) -> Result<Option<Value>, String> {
        require_name(name)?;
        read_json(&self.maps_src().join(format!("{name}{MAP_SUFFIX}")))
    }

    /// Writes the export, then the map in the schema from it, unless the crate refuses it:
    /// {"saved": the export's path, "import": what writing the map did}.
    pub fn save_map(&self, name: &str, export: &Value) -> Result<Value, String> {
        require_name(name)?;
        if export.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err(format!("body is not a designer map (format must be \"{FORMAT}\")"));
        }
        let source = self.maps_src().join(format!("{name}{MAP_SUFFIX}"));
        write_json(&source, export)?;
        Ok(
            json!({"saved": format!("content/maps_src/{name}{MAP_SUFFIX}"), "import": self.write_map(name, export)}),
        )
    }

    fn write_map(&self, name: &str, export: &Value) -> Value {
        let library: TerrainLibrary = match load_file(&self.root.join(TERRAIN_LIBRARY)) {
            Ok(library) => library,
            Err(error) => return outcome(Vec::new(), vec![error.to_string()], None),
        };
        let read = map_from_export(export, &library);
        let mut errors = read.errors;
        if errors.is_empty() {
            errors.extend(read.map.check(&library).err());
        }
        if errors.is_empty() {
            if let Err(error) = write_text(&self.map_out(name), &file_text(&read.map)) {
                errors.push(error);
            }
        }
        outcome(read.warnings, errors, Some(format!("content/maps/{name}.json")))
    }

    /// Opens the Godot map viewer (specs/18) on map `name`.
    pub fn view_map(&self, name: &str) -> Result<Value, String> {
        require_name(name)?;
        let Some(godot) = &self.godot else {
            return Err("Godot binary not configured, so the viewer can't be opened".into());
        };
        if !self.map_out(name).is_file() {
            return Err(format!("content/maps/{name}.json doesn't exist yet - save the map first"));
        }
        let root = self.root.display().to_string();
        let command = [
            godot.clone(),
            "--path".into(),
            root,
            VIEWER_SCENE.into(),
            "--".into(),
            format!("--map={name}"),
        ];
        (self.launcher)(&command)
            .map_err(|error| format!("could not start Godot ({godot}): {error}"))?;
        Ok(json!({"launched": true, "map": format!("content/maps/{name}.json")}))
    }

    /// A library; null if it has no file yet. The terrain library is read into the
    /// designer's shape.
    pub fn read_library(&self, library: &str) -> Result<Value, String> {
        require_library(library)?;
        if library != "terrain" {
            return Ok(read_json(&self.library_path(library))?.unwrap_or(Value::Null));
        }
        let path = self.root.join(TERRAIN_LIBRARY);
        if !path.is_file() {
            return Ok(Value::Null);
        }
        let stored: TerrainLibrary = load_file(&path).map_err(|error| error.to_string())?;
        Ok(terrain::from_schema(&stored))
    }

    /// Writes a library: {"changed", "import"}. The terrain library is written in the schema,
    /// and refused (nothing written, "import" saying why) if it drops a terrain or feature a
    /// saved map uses, or doesn't fit the schema.
    pub fn save_library(&self, library: &str, value: &Value) -> Result<Value, String> {
        require_library(library)?;
        if library != "terrain" {
            let changed = write_json(&self.library_path(library), value)?;
            return Ok(json!({"changed": changed, "import": null}));
        }
        let problems = match terrain::to_schema(value) {
            Ok(schema) => match schema.missing_material() {
                Some((field, id)) => Err(vec![format!("{field}: no material {id}")]),
                None => {
                    let in_use = terrain::in_use_problems(&schema, &self.maps_src());
                    if in_use.is_empty() {
                        Ok(schema)
                    } else {
                        Err(in_use)
                    }
                }
            },
            Err(error) => Err(vec![error]),
        };
        let (changed, errors) = match problems {
            Ok(schema) => {
                (write_text(&self.root.join(TERRAIN_LIBRARY), &file_text(&schema))?, Vec::new())
            }
            Err(errors) => (false, errors),
        };
        Ok(json!({"changed": changed, "import": outcome(Vec::new(), errors, None)}))
    }

    fn library_path(&self, library: &str) -> PathBuf {
        self.content().join("designer").join(format!("{library}.json"))
    }
}

fn require_name(name: &str) -> Result<(), String> {
    if is_valid_name(name) {
        Ok(())
    } else {
        Err(format!("invalid map name {name:?}: use letters, digits, '_' and '-' only"))
    }
}

fn require_library(library: &str) -> Result<(), String> {
    if LIBRARIES.contains(&library) {
        Ok(())
    } else {
        Err(format!("unknown library {library:?}"))
    }
}

/// Starts `command` apart from the server, so closing either leaves the other.
pub fn launch_detached(command: &[String]) -> std::io::Result<()> {
    let mut process = std::process::Command::new(&command[0]);
    process
        .args(&command[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x8;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x200;
        process.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    process.spawn().map(|_| ())
}
