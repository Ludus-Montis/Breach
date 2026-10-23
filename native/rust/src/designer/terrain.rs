//! The designer's terrain library and the schema's (spec 39 step 2b, moved into the crate in
//! 2c). The designer edits the library in its own shape (label, relief {amplitude, scale},
//! strata [{material, min, max}], heat transitions [{above|below, becomes|gains}]); the file
//! on disk, `content/terrain/terrain_library.json`, is the schema's `TerrainLibrary`.
//! `to_schema` reads the designer's, `from_schema` gives it back. The rules are the ones the
//! Godot library import had, saves from before Decisions 62 to 66 included.
//!
//! `in_use_problems` keeps a library edit from breaking a map: every terrain and feature a
//! saved designer map (`content/maps_src/*.designer.json`) uses must still be in the library.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use super::{field_text, flag, int, list, real, text};
use crate::content::{
    Colour, HeatTransition, LiquidBody, Material, Stratum, Terrain, TerrainFeature, TerrainLibrary,
};

/// A saved designer map's file name ends in this.
pub const MAP_SUFFIX: &str = ".designer.json";
const CAPACITY: [&str; 5] = ["stability", "max_height", "max_width", "max_length", "max_depth"];

/// The designer's library in the schema; an error for an entry with no id.
pub fn to_schema(library: &Value) -> Result<TerrainLibrary, String> {
    check_colours(library)?;
    let mut materials: Vec<Material> =
        entries(library, "materials", "material")?.iter().map(material).collect();
    // A liquid library saved before Decision 64: each a material that flows (lava slowly).
    for entry in entries(library, "liquids", "liquid")? {
        let mut liquid = material(entry);
        let flows = if liquid.heat > 4 { 1 } else { 3 };
        liquid.traits.entry("flows".into()).or_insert(flows);
        materials.push(liquid);
    }
    Ok(TerrainLibrary {
        terrains: entries(library, "terrains", "terrain")?.iter().map(terrain).collect(),
        features: entries(library, "features", "feature")?.iter().map(feature).collect(),
        materials,
        road_move_multiplier: library.get("road_move_multiplier").map_or(0.5, real),
    })
}

/// The schema's library in the designer's shape.
pub fn from_schema(library: &TerrainLibrary) -> Value {
    json!({
        "terrains": library.terrains.iter().map(designer_terrain).collect::<Vec<_>>(),
        "materials": library.materials.iter().map(designer_material).collect::<Vec<_>>(),
        "features": library.features.iter().map(|entry| json!({
            "id": entry.id,
            "label": entry.display_name,
            "glyph": entry.glyph,
            "stub_effect": entry.effect_notes,
        })).collect::<Vec<_>>(),
        "road_move_multiplier": library.road_move_multiplier,
    })
}

/// Each terrain (then each feature) id a saved map in `maps_src` uses, with the maps using
/// it, in the order they were first found.
pub type Used = Vec<(String, Vec<String>)>;

/// The terrains and the features the saved maps in `maps_src` use.
pub fn used_ids(maps_src: &Path) -> (Used, Used) {
    let (mut terrains, mut features) = (Used::new(), Used::new());
    let mut paths: Vec<_> = fs::read_dir(maps_src)
        .map(|entries| entries.filter_map(|entry| entry.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    paths.retain(|path| path.to_string_lossy().ends_with(MAP_SUFFIX));
    paths.sort();
    for path in paths {
        let read = fs::read_to_string(&path).ok();
        let Some(export) = read.and_then(|text| serde_json::from_str::<Value>(&text).ok()) else {
            continue;
        };
        if !export.is_object() {
            continue;
        }
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        let name = file.trim_end_matches(MAP_SUFFIX).to_string();
        let mut terrain_ids = vec![field_text(&export, "default_terrain")];
        let mut feature_ids = Vec::new();
        for tile in list(&export, "tiles") {
            if !tile.get("terrain_is_base").is_some_and(flag) {
                terrain_ids.push(field_text(tile, "terrain"));
            }
            if let Some(feature) = tile.get("feature").filter(|feature| !feature.is_null()) {
                feature_ids.push(text(feature));
            }
        }
        note(&mut terrains, terrain_ids, &name);
        note(&mut features, feature_ids, &name);
    }
    (terrains, features)
}

fn note(used: &mut Used, ids: Vec<String>, name: &str) {
    for id in ids.into_iter().filter(|id| !id.is_empty()) {
        let at = match used.iter().position(|(known, _)| *known == id) {
            Some(at) => at,
            None => {
                used.push((id, Vec::new()));
                used.len() - 1
            }
        };
        if !used[at].1.iter().any(|map| map == name) {
            used[at].1.push(name.to_string());
        }
    }
}

/// Each terrain or feature a saved map in `maps_src` uses that `library` lacks.
pub fn in_use_problems(library: &TerrainLibrary, maps_src: &Path) -> Vec<String> {
    let (terrains, features) = used_ids(maps_src);
    let has_terrain = |id: &str| library.terrains.iter().any(|terrain| terrain.id == id);
    let has_feature = |id: &str| library.features.iter().any(|feature| feature.id == id);
    let missing = |kind: &str, used: &Used, has: &dyn Fn(&str) -> bool| -> Vec<String> {
        used.iter()
            .filter(|(id, _)| !has(id))
            .map(|(id, maps)| {
                format!(
                    "{kind} '{id}' is still used by {} - change those maps first, or keep it \
                     in the library",
                    maps.join(", ")
                )
            })
            .collect()
    };
    let mut problems = missing("terrain", &terrains, &has_terrain);
    problems.extend(missing("feature", &features, &has_feature));
    problems
}

/// Fails naming the first terrain or material whose colour isn't `#rrggbb` or `#rrggbbaa`.
fn check_colours(library: &Value) -> Result<(), String> {
    for key in ["terrains", "materials", "liquids"] {
        for entry in list(library, key) {
            if let Some(written) = entry.get("color") {
                Colour::try_from(text(written))
                    .map_err(|why| format!("{} {}: color: {why}", key, field_text(entry, "id")))?;
            }
        }
    }
    Ok(())
}

fn entries<'a>(library: &'a Value, key: &str, kind: &str) -> Result<&'a [Value], String> {
    let found = list(library, key);
    match found.iter().find(|entry| field_text(entry, "id").is_empty()) {
        Some(nameless) => Err(format!("a {kind} entry has no id: {nameless}")),
        None => Ok(found),
    }
}

/// `entry[key]` as a whole number, or `default`.
fn int_at(entry: &Value, key: &str, default: i64) -> i64 {
    entry.get(key).map_or(default, int)
}

fn label(entry: &Value) -> String {
    entry.get("label").map_or_else(|| field_text(entry, "id"), text)
}

/// A checked colour (`check_colours`), white if none is written.
fn colour(entry: &Value) -> Colour {
    entry.get("color").and_then(|written| Colour::try_from(text(written)).ok()).unwrap_or_default()
}

fn terrain(entry: &Value) -> Terrain {
    let relief =
        entry.get("relief").filter(|relief| relief.is_object()).cloned().unwrap_or(json!({}));
    let bearing = int_at(entry, "bearing", 0);
    let blocked = entry.get("blocks_unit_classes").map_or(String::new(), text);
    let liquids = match entry.get("liquids") {
        Some(found) => found.as_array().cloned().unwrap_or_default(),
        None => legacy_water(entry),
    };
    let capacity = |field: &str| int_at(entry, &format!("default_{field}"), 0);
    let [stability, max_height, max_width, max_length, max_depth] = CAPACITY.map(capacity);
    Terrain {
        id: field_text(entry, "id"),
        display_name: label(entry),
        glyph: field_text(entry, "glyph"),
        color: colour(entry),
        can_be_base: entry.get("can_be_base").is_none_or(flag),
        needs_bridge: entry.get("needs_bridge").is_some_and(flag),
        move_cost: entry.get("move_cost").map_or(1.0, real),
        blocks_unit_classes: blocked
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(String::from)
            .collect(),
        bearing,
        default_elevation: int_at(entry, "default_elevation", 0),
        relief_amplitude: int_at(&relief, "amplitude", 0),
        relief_scale: int_at(&relief, "scale", 0),
        foundation_max: int_at(entry, "foundation_max", bearing),
        dig_depth: int_at(entry, "dig_depth", 0),
        liquids: liquids.iter().map(liquid).collect(),
        strata: list(entry, "strata").iter().map(stratum).collect(),
        default_stability: stability,
        default_max_height: max_height,
        default_max_width: max_width,
        default_max_length: max_length,
        default_max_depth: max_depth,
    }
}

fn liquid(body: &Value) -> LiquidBody {
    let least = int_at(body, "min", 0);
    let material = body.get("material").or_else(|| body.get("liquid"));
    LiquidBody {
        material_id: material.map_or(String::new(), text),
        min_cells: least,
        max_cells: int_at(body, "max", least),
        chance: body.get("chance").map_or(1.0, real),
        surface_chance: body.get("surface_chance").map_or(0.0, real),
    }
}

fn stratum(band: &Value) -> Stratum {
    let least = int_at(band, "min", 1);
    Stratum {
        material_id: field_text(band, "material"),
        min_cells: least,
        max_cells: int_at(band, "max", least),
    }
}

/// A water table saved before Decision 62, as a water body that is always there.
fn legacy_water(entry: &Value) -> Vec<Value> {
    match entry.get("water_table").filter(|water| water.is_object()) {
        Some(water) => vec![json!({
            "material": "WATER",
            "min": water.get("min").cloned().unwrap_or(json!(0)),
            "max": water.get("max").cloned().unwrap_or(json!(0)),
        })],
        None => Vec::new(),
    }
}

fn material(entry: &Value) -> Material {
    let heat = match entry.get("heat") {
        Some(heat) => int(heat),
        None => heat_of(int_at(entry, "temperature", 15)),
    };
    let mut traits: std::collections::BTreeMap<String, i64> = entry
        .get("traits")
        .and_then(Value::as_object)
        .map(|traits| traits.iter().map(|(key, value)| (key.clone(), int(value))).collect())
        .unwrap_or_default();
    // Saved before Decisions 63-64: a loose flag, and difficulties as fields.
    if entry.get("loose").is_some_and(flag) {
        traits.insert("loose".into(), 1);
    }
    for field in ["dig_difficulty", "climb_difficulty"] {
        if int_at(entry, field, 0) > 0 {
            traits.insert(field.into(), int_at(entry, field, 0));
        }
    }
    Material {
        id: field_text(entry, "id"),
        display_name: label(entry),
        color: colour(entry),
        weight: int_at(entry, "weight", 1),
        span: int_at(entry, "span", 1),
        strength: int_at(entry, "strength", 0),
        heat,
        traits,
        heat_transitions: list(entry, "heat_transitions").iter().map(transition).collect(),
    }
}

/// {"above"|"below": heat level, "becomes": id | "gains": trait} (Decisions 63, 66). A
/// threshold over 10 is degrees, saved before Decision 66.
fn transition(rule: &Value) -> HeatTransition {
    let threshold = rule.get("above").or_else(|| rule.get("below")).map_or(0, int);
    HeatTransition {
        threshold: if threshold > 10 { heat_of(threshold) } else { threshold },
        rising: rule.get("above").is_some(),
        becomes: field_text(rule, "becomes"),
        gains_trait: field_text(rule, "gains"),
    }
}

/// A temperature in degrees, saved before Decision 66, as a heat level 0-10.
fn heat_of(degrees: i64) -> i64 {
    if degrees <= 50 {
        1
    } else {
        10.min((degrees as f64 / 150.0).ceil() as i64)
    }
}

fn feature(entry: &Value) -> TerrainFeature {
    TerrainFeature {
        id: field_text(entry, "id"),
        display_name: label(entry),
        glyph: field_text(entry, "glyph"),
        effect_notes: field_text(entry, "stub_effect"),
    }
}

fn designer_terrain(entry: &Terrain) -> Value {
    let mut terrain = serde_json::to_value(entry).unwrap_or(json!({}));
    let Some(fields) = terrain.as_object_mut() else {
        return terrain;
    };
    for renamed in ["display_name", "relief_amplitude", "relief_scale"] {
        fields.remove(renamed);
    }
    fields.insert("label".into(), json!(entry.display_name));
    fields.insert("blocks_unit_classes".into(), json!(entry.blocks_unit_classes.join(", ")));
    fields.insert(
        "relief".into(),
        json!({"amplitude": entry.relief_amplitude, "scale": entry.relief_scale}),
    );
    let strata: Vec<Value> = entry
        .strata
        .iter()
        .map(|band| json!({"material": band.material_id, "min": band.min_cells, "max": band.max_cells}))
        .collect();
    fields.insert("strata".into(), json!(strata));
    let liquids: Vec<Value> = entry
        .liquids
        .iter()
        .map(|body| {
            json!({
                "material": body.material_id,
                "min": body.min_cells,
                "max": body.max_cells,
                "chance": body.chance,
                "surface_chance": body.surface_chance,
            })
        })
        .collect();
    fields.insert("liquids".into(), json!(liquids));
    terrain
}

fn designer_material(entry: &Material) -> Value {
    let mut material = serde_json::to_value(entry).unwrap_or(json!({}));
    if let Some(fields) = material.as_object_mut() {
        fields.remove("display_name");
        fields.insert("label".into(), json!(entry.display_name));
        let rules: Vec<Value> = entry.heat_transitions.iter().map(designer_transition).collect();
        fields.insert("heat_transitions".into(), json!(rules));
    }
    material
}

fn designer_transition(rule: &HeatTransition) -> Value {
    let mut out = serde_json::Map::new();
    out.insert((if rule.rising { "above" } else { "below" }).into(), json!(rule.threshold));
    if !rule.becomes.is_empty() {
        out.insert("becomes".into(), json!(rule.becomes));
    }
    if !rule.gains_trait.is_empty() {
        out.insert("gains".into(), json!(rule.gains_trait));
    }
    Value::Object(out)
}
