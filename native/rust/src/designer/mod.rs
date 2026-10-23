//! The map designer's side of the crate (spec 39 step 2c, Decision 153). The designer's page
//! (`tools/designer/`) saves a map as its own export (`content/maps_src/<name>.designer.json`,
//! which it reopens) and edits the terrain library in its own shape; this module reads them
//! into the schema's `Map` and `TerrainLibrary`, and the library back. Its local server is
//! here too (`http.rs`, `repo.rs`, `registry.rs`), run by the `designer` binary
//! (`src/bin/designer.rs`, `native/bin/breach_designer`).
//!
//! The rules are the ones Godot's designer import had (specs 16 and 19): a map's lanes come
//! from its links (`lanes.rs`), its nodes from the export's nodes (`nodes.rs`) and its layout
//! from the grid, tiles, roads, links' routes and channels (`layout.rs`). The export is read
//! as loosely as that import read it: a missing value takes its default and a number may be
//! a string.

pub mod http;
mod lanes;
mod layout;
mod nodes;
mod plan;
pub mod registry;
pub mod repo;
pub mod terrain;

use serde_json::Value;

use crate::content::{Faction, FactionRelation, LossGroup, LossRule, Map, Stance, TerrainLibrary};

/// The export's format, and the version this reads.
pub const FORMAT: &str = "breach-designer-map";
pub const FORMAT_VERSION: i64 = 1;

/// The designer has no tick scalars: a map takes p_f_F_c's unless its export has "sim".
const DEFAULT_TICK_SECONDS: f64 = 1.5;
const DEFAULT_THRESHOLDS: [i64; 4] = [20, 45, 70, 90];
const DEFAULT_DECAY: i64 = 2;
/// Sections of the export the schema doesn't hold yet (spec 20).
const NOT_YET_IMPORTED: [&str; 2] = ["static_defense_library", "structure_feature_library"];

/// A map read from the designer's export, and what was wrong with the export.
#[derive(Debug, Default)]
pub struct Translated {
    pub map: Map,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// The designer's `export` as a map, its terrain and features checked against `library`.
pub fn map_from_export(export: &Value, library: &TerrainLibrary) -> Translated {
    let mut out = Translated::default();
    let format = export.get("format");
    let version = export.get("format_version");
    if format.and_then(Value::as_str) != Some(FORMAT) || version.map(int) != Some(FORMAT_VERSION) {
        out.errors.push(format!(
            "unsupported export format: expected '{FORMAT}' version {FORMAT_VERSION}, got '{}' \
             version {}",
            text(format.unwrap_or(&Value::Null)),
            text(version.unwrap_or(&Value::Null)),
        ));
        return out;
    }
    let map = &mut out.map;
    apply_sim(export.get("sim").unwrap_or(&Value::Null), map);
    map.factions = list(export, "factions")
        .iter()
        .map(|entry| Faction {
            id: field_text(entry, "id"),
            display_name: field_text(entry, "display_name"),
        })
        .collect();
    map.faction_relations = list(export, "faction_relations")
        .iter()
        .map(|entry| relation(entry, &mut out.errors))
        .collect();
    map.nodes = nodes::build(export, &mut out.errors, &mut out.warnings);
    lanes::derive(export, map, &mut out.errors);
    map.layout = layout::build(export, library, &mut out.errors);
    map.loss_groups = list(export, "loss_criteria")
        .iter()
        .map(|entry| loss_group(entry, &mut out.errors))
        .collect();
    warn_not_imported(export, &mut out.warnings);
    out
}

/// A designer structure plan (spec 24) as the schema's, and what was wrong with it.
pub fn plan_from_export(drawn: &Value) -> (crate::content::StructurePlan, Vec<String>) {
    let mut errors = Vec::new();
    (plan::build(drawn, "", &mut errors), errors)
}

fn apply_sim(sim: &Value, map: &mut Map) {
    map.tick_duration_seconds = sim.get("tick_duration_seconds").map_or(DEFAULT_TICK_SECONDS, real);
    map.suspicion_tier_thresholds = match sim.get("suspicion_tier_thresholds") {
        Some(values) => {
            values.as_array().map_or(Vec::new(), |values| values.iter().map(int).collect())
        }
        None => DEFAULT_THRESHOLDS.to_vec(),
    };
    map.suspicion_decay_per_tick = sim.get("suspicion_decay_per_tick").map_or(DEFAULT_DECAY, int);
}

fn relation(entry: &Value, errors: &mut Vec<String>) -> FactionRelation {
    let mut relation = FactionRelation {
        faction_a_id: field_text(entry, "a"),
        faction_b_id: field_text(entry, "b"),
        ..FactionRelation::default()
    };
    let stance = field_text(entry, "stance");
    match named::<Stance>(&stance) {
        Some(named) => relation.stance = named,
        None => errors.push(format!(
            "relation {}-{}: unknown stance '{stance}'",
            relation.faction_a_id, relation.faction_b_id
        )),
    }
    relation
}

fn loss_group(entry: &Value, errors: &mut Vec<String>) -> LossGroup {
    let mut group = LossGroup {
        faction_id: field_text(entry, "faction_id"),
        display_name: field_text(entry, "name"),
        node_ids: strings(entry.get("node_ids")),
        ..LossGroup::default()
    };
    let rule = entry.get("rule").map_or("ANY".to_string(), text);
    match named::<LossRule>(&rule) {
        Some(named) => group.rule = named,
        None => errors.push(format!(
            "loss group '{}' ({}): unknown rule '{rule}'",
            group.display_name, group.faction_id
        )),
    }
    group
}

fn warn_not_imported(export: &Value, warnings: &mut Vec<String>) {
    let mut skipped: Vec<&str> =
        NOT_YET_IMPORTED.into_iter().filter(|section| !list(export, section).is_empty()).collect();
    if list(export, "nodes").iter().any(|node| node.get("structure").is_some()) {
        skipped.push("structures");
    }
    if !skipped.is_empty() {
        warnings.push(format!("not imported yet (spec 20): {}", skipped.join(", ")));
    }
}

/// The value of an enum named `name` (as the schema writes it), if it is one.
fn named<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_value(Value::String(name.to_string())).ok()
}

/// `value[key]` as a list, or none.
fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// `value[key]` as text: none and null are "".
fn field_text(value: &Value, key: &str) -> String {
    value.get(key).map_or(String::new(), text)
}

/// A value as text: null is "", a string itself, anything else its JSON.
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// A list of values as text; none if it isn't a list.
fn strings(values: Option<&Value>) -> Vec<String> {
    values.and_then(Value::as_array).map_or(Vec::new(), |values| values.iter().map(text).collect())
}

/// A value as a whole number: a real truncates, a string is read, true is 1.
fn int(value: &Value) -> i64 {
    match value {
        Value::Number(number) => {
            number.as_i64().unwrap_or_else(|| number.as_f64().unwrap_or(0.0) as i64)
        }
        Value::Bool(flag) => i64::from(*flag),
        Value::String(text) => text.trim().parse::<f64>().map_or(0, |read| read as i64),
        _ => 0,
    }
}

/// A value as a real: a string is read, true is 1.
fn real(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64().unwrap_or(0.0),
        Value::Bool(flag) => f64::from(u8::from(*flag)),
        Value::String(text) => text.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// A value as a flag: a non-zero number or a non-empty string is true.
fn flag(value: &Value) -> bool {
    match value {
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|read| read != 0.0),
        Value::String(text) => !text.is_empty(),
        _ => false,
    }
}

/// `value[key]` as a whole number, or `default` if it is missing or null.
fn int_or(value: &Value, key: &str, default: i64) -> i64 {
    value.get(key).filter(|found| !found.is_null()).map_or(default, int)
}

/// `value[key]` as a flag, false if it is missing.
fn flag_of(value: &Value, key: &str) -> bool {
    value.get(key).is_some_and(flag)
}

#[cfg(test)]
mod server_tests;
#[cfg(test)]
mod tests;
