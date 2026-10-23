//! The content registry the designer's Library page browses (Decision 128; in Rust since spec
//! 39 step 2c): the tags, traits and statuses (`content/registry/{tags,traits,statuses}.json`),
//! where each tag and trait is used across the repo's content (the simulation's JSON under
//! `content/units`, `weapons` and `armour`, the `.tres` left under `content/`, and the
//! designer's own libraries and the terrain library), and each unit's traits. A trait's
//! targets - the kinds of thing it may sit on - follow Godot's ContentRegistry: where all its
//! tags' targets agree, narrowed by its "only". A trait with a "default" is implicit: every unit
//! has it at that level without listing it, and the units list it, labelled implicit.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

const KINDS: [&str; 3] = ["tags", "traits", "statuses"];
const DEFINITIONS: [&str; 3] = ["units", "weapons", "armour"];

/// {"tags", "traits", "statuses", "usage": {id: [paths]}, "implicit": {id: level},
/// "units": [...]}, each trait with its "targets".
pub fn read_registry(root: &Path) -> Value {
    let mut out = Map::new();
    for kind in KINDS {
        let path = root.join("content/registry").join(format!("{kind}.json"));
        let read: Option<Value> =
            fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok());
        out.insert(
            kind.into(),
            read.and_then(|value| value.get(kind).cloned()).unwrap_or(json!([])),
        );
    }
    let tags: BTreeMap<String, Value> = out["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| Some((tag.get("id")?.as_str()?.to_string(), tag.clone())))
        .collect();
    if let Some(traits) = out.get_mut("traits").and_then(Value::as_array_mut) {
        for entry in traits.iter_mut() {
            let reach = targets(entry, &tags);
            if let Some(fields) = entry.as_object_mut() {
                fields.insert("targets".into(), json!(reach));
            }
        }
    }
    let implicit: BTreeMap<String, i64> = out["traits"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some((entry.get("id")?.as_str()?.to_string(), entry.get("default")?.as_f64()? as i64))
        })
        .collect();
    out.insert("usage".into(), json!(usage(root)));
    out.insert("units".into(), read_units(root, &implicit));
    out.insert("implicit".into(), json!(implicit));
    Value::Object(out)
}

/// Each unit under `content/units`, by id: {"id", "path", "traits": [{"id", "level",
/// "implicit"}]}: the traits it lists, then each implicit one it doesn't, at its default.
pub fn read_units(root: &Path, implicit: &BTreeMap<String, i64>) -> Value {
    let units = sorted_files(&root.join("content/units"), "json").into_iter().map(|path| {
        let read: Value = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let own: BTreeMap<String, Value> = read
            .get("traits")
            .and_then(Value::as_object)
            .map(|traits| traits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let mut traits: Vec<Value> = own
            .iter()
            .map(|(id, level)| json!({"id": id, "level": level, "implicit": false}))
            .collect();
        for (id, level) in implicit.iter().filter(|(id, _)| !own.contains_key(*id)) {
            traits.push(json!({"id": id, "level": level, "implicit": true}));
        }
        let id = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        json!({"id": id, "path": relative(root, &path), "traits": traits})
    });
    Value::Array(units.collect())
}

/// The kinds of thing a trait may sit on.
pub fn targets(entry: &Value, tags: &BTreeMap<String, Value>) -> Vec<String> {
    let mut reach: Option<Vec<String>> = None;
    for tag in entry.get("tags").and_then(Value::as_array).into_iter().flatten() {
        let kinds: Vec<String> = tag
            .as_str()
            .and_then(|tag| tags.get(tag))
            .and_then(|tag| tag.get("targets"))
            .and_then(Value::as_array)
            .map(|kinds| kinds.iter().filter_map(|k| k.as_str().map(String::from)).collect())
            .unwrap_or_default();
        reach = Some(match reach {
            None => kinds,
            Some(sofar) => sofar.into_iter().filter(|kind| kinds.contains(kind)).collect(),
        });
    }
    let mut reach = reach.unwrap_or_default();
    if let Some(only) = entry.get("only").and_then(Value::as_array) {
        reach.retain(|kind| only.iter().any(|allowed| allowed.as_str() == Some(kind)));
    }
    reach
}

/// Tag or trait id -> the content files (repo-relative) that use it.
fn usage(root: &Path) -> BTreeMap<String, Vec<String>> {
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut note = |ids: Vec<String>, path: &Path| {
        let mut ids = ids;
        ids.sort();
        ids.dedup();
        for id in ids {
            found.entry(id).or_default().push(relative(root, path));
        }
    };
    let content = root.join("content");
    for path in all_files(&content, "tres") {
        note(tres_ids(&fs::read_to_string(&path).unwrap_or_default()), &path);
    }
    for kind in DEFINITIONS {
        for path in sorted_files(&content.join(kind), "json") {
            note(definition_ids(&read_value(&path)), &path);
        }
    }
    let mut libraries = sorted_files(&content.join("designer"), "json");
    libraries.extend(sorted_files(&content.join("terrain"), "json"));
    for path in libraries {
        note(designer_traits(&read_value(&path)), &path);
    }
    found
}

fn read_value(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The ids in a `.tres`: the keys of its `traits = {...}` and `grants = {...}` blocks and the
/// words of its `tags = Array[String]([...])`.
fn tres_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("traits = {") || line.starts_with("grants = {") {
            let mut block = line.to_string();
            if !line.trim_end().ends_with('}') {
                for inner in lines.by_ref() {
                    if inner.starts_with('}') {
                        break;
                    }
                    block.push_str(inner);
                }
            }
            ids.extend(
                quoted(&block).into_iter().filter(|(_, colon)| *colon).map(|(word, _)| word),
            );
        } else if let Some(rest) = line.strip_prefix("tags = Array[String]([") {
            let inside = rest.split("])").next().unwrap_or_default();
            ids.extend(quoted(inside).into_iter().map(|(word, _)| word));
        }
    }
    ids
}

/// Each `"word"` of lowercase letters and `_` in `text`, and whether a `:` follows it.
fn quoted(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let word = &after[..close];
        let tail = &after[close + 1..];
        if !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            out.push((word.to_string(), tail.trim_start().starts_with(':')));
        }
        rest = tail;
    }
    out
}

/// The trait ids (its own and those it grants) and tags a definition names, at any depth.
fn definition_ids(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Value::Object(fields) = value {
        for (key, inner) in fields {
            match (key.as_str(), inner) {
                ("traits" | "grants", Value::Object(traits)) => out.extend(traits.keys().cloned()),
                ("tags", Value::Array(tags)) => {
                    out.extend(tags.iter().filter_map(|t| t.as_str().map(String::from)))
                }
                _ => out.extend(definition_ids(inner)),
            }
        }
    }
    out
}

/// Trait ids under any "traits" object in a designer library.
fn designer_traits(value: &Value) -> Vec<String> {
    match value {
        Value::Object(fields) => fields
            .iter()
            .flat_map(|(key, inner)| match (key.as_str(), inner) {
                ("traits", Value::Object(traits)) => traits.keys().cloned().collect(),
                _ => designer_traits(inner),
            })
            .collect(),
        Value::Array(items) => items.iter().flat_map(designer_traits).collect(),
        _ => Vec::new(),
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// The files in `dir` with extension `ext`, in name order.
fn sorted_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.filter_map(|entry| entry.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    paths.retain(|path| path.is_file() && path.extension().is_some_and(|found| found == ext));
    paths.sort();
    paths
}

/// The files under `dir`, at any depth, with extension `ext`, in path order.
fn all_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut found = sorted_files(dir, ext);
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.filter_map(|entry| entry.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    dirs.retain(|path| path.is_dir());
    for sub in dirs {
        found.extend(all_files(&sub, ext));
    }
    found.sort();
    found
}
