//! Breach's native core (Decision 129, spec 39): the whole simulation, through GDExtension
//! with godot-rust. The scenes read it through its bindings (`BattleState`, `FieldState`,
//! `LaneState`, `EconomyState`, `SimulationClock`, `SkirmishClock`) and the content through
//! `ContentStore`; CI checks it with the crate's binaries (`breach_digest`, `breach_trials`
//! and the benches). No GDScript plays the tick since spec 39 step 9d-iv.

mod aftermath;
mod arms;
mod bands;
mod battle_play;
mod battles;
mod blows;
#[cfg(test)]
mod build;
mod carry;
mod clock;
mod columns;
mod commands;
mod compact;
mod contact;
pub mod content;
mod contest;
pub mod cuts;
mod deaths;
pub mod designer;
mod det_math;
mod det_pow;
mod detours;
mod digest;
/// The digest's runs, for the digest binary (src/bin/digest.rs).
pub use digest::runs as digest_runs;
/// The digest's trace, for the digest binary.
pub use digest::trace as digest_trace;
mod discipline;
mod economy;
mod ending;
mod engage;
mod entries;
mod events;
mod face_store;
mod facing;
mod field;
mod field_parting;
mod field_play;
mod fight;
mod frame;
mod geometry;
mod ghash;
mod grid;
pub mod ground;
mod groups;
mod hurts;
mod joins;
mod kingdom;
mod landing;
mod lane;
mod laps;
mod locks;
mod lying;
mod made;
mod manoeuvre;
mod march;
mod maths;
mod morale;
mod motion;
mod mover;
mod moving;
mod mustering;
pub mod native_bench;
mod near;
mod objects;
mod opening;
mod ordered;
mod orders;
mod parting;
mod pathing;
mod paths;
mod perception;
pub mod phases;
mod play;
mod pursuit;
mod queue;
mod ranks;
mod reach;
mod record;
mod recovery;
mod rect;
mod regeneration;
mod regroup;
mod rehearsal;
mod rendezvous;
mod roster;
mod rout;
pub mod rout_bench;
mod rout_blows;
mod route;
pub mod scaling;
mod scrum;
mod seeking;
mod sheet;
mod shocks;
mod shots;
mod snapshot;
mod spawn;
mod split;
mod squads;
mod staging;
mod stamina;
mod stance;
mod steer;
mod strays;
pub mod structure;
mod swaps;
mod taking;
mod tick;
mod trade;
pub mod trials;
mod turn;
mod walk_store;
mod walking;
mod waves;
mod withdraw;
mod words;
mod wounds;

use godot::prelude::*;

struct BreachNative;

#[gdextension]
unsafe impl ExtensionLibrary for BreachNative {}

/// The simulation's content as the crate loaded it (spec 39 step 2, Decision 153), handed to
/// GDScript as its definitions (`UnitDef`, `MapDef`...), filled field for field by name, until
/// their last reader ports. Each is made once and shared, as `load()` shared a `.tres`. Also
/// what reads and writes a map file and reads the designer's exports for GDScript. CrateContent
/// is GDScript's way in.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct ContentStore {
    content: Option<content::Content>,
    made: std::collections::BTreeMap<String, Gd<Object>>,
}

#[godot_api]
impl ContentStore {
    /// Loads the content under `root` (an OS path to the repo's `content/`), once: "" when
    /// loaded, else why not.
    #[func]
    fn load(&mut self, root: GString) -> GString {
        if self.content.is_some() {
            return GString::new();
        }
        match content::Content::load(std::path::Path::new(&root.to_string())) {
            Ok(loaded) => {
                self.content = Some(loaded);
                GString::new()
            }
            Err(error) => GString::from(&error.to_string()),
        }
    }

    /// The unit `id`, its items the shared ones; null if there is none.
    #[func]
    fn unit(&mut self, id: GString) -> Variant {
        let key = format!("units/{id}");
        if let Some(made) = self.made.get(&key) {
            return made.to_variant();
        }
        let Some(unit) = self.content.as_ref().and_then(|c| c.units.get(&id.to_string())) else {
            return Variant::nil();
        };
        let mut fields = json_fields(unit);
        let paths = fields.remove("items");
        let made = made_from("UnitDef", &fields);
        let items: VarArray = paths
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .map(|path| self.item(path.as_str().unwrap_or_default()))
            .collect();
        made.get("items").call("assign", &[items.to_variant()]);
        self.made.insert(key, made.clone());
        made.to_variant()
    }

    /// The weapon `id`; null if there is none.
    #[func]
    fn weapon(&mut self, id: GString) -> Variant {
        self.item(&format!("weapons/{id}"))
    }

    /// The armour `id`; null if there is none.
    #[func]
    fn armour(&mut self, id: GString) -> Variant {
        self.item(&format!("armour/{id}"))
    }

    /// The battle's tuning.
    #[func]
    fn tuning(&mut self) -> Variant {
        let fields = self.content.as_ref().map(|c| json_fields(&c.tuning));
        self.made_once("tuning", "BattleTuning", fields)
    }

    /// The one shared terrain library (Decision 32).
    #[func]
    fn terrain_library(&mut self) -> Variant {
        let fields = self.content.as_ref().map(|c| json_fields(&c.terrain));
        self.made_once("terrain", "TerrainLibraryDef", fields)
    }

    /// The map `name` (content/maps/<name>.json); null if there is none. Its layout's
    /// terrain library is CrateContent's, so make that first.
    #[func]
    fn map(&mut self, name: GString) -> Variant {
        let key = format!("maps/{name}");
        if let Some(made) = self.made.get(&key) {
            return made.to_variant();
        }
        let Some(map) = self.content.as_ref().and_then(|c| c.maps.get(&name.to_string())) else {
            return Variant::nil();
        };
        let made = map_object(map);
        self.made.insert(key, made.clone());
        made.to_variant()
    }

    /// A unit of `faction` made in the crate from the definition `id`, advancing in
    /// `direction` (spec 39 step 8c-iii, `made::unit`), for the test that holds it equal to
    /// FormationUnits.make's: {"sheet": its numbers by `sheet_names`, "words": its words and
    /// lists by name, "walker": its walker's fields}. Empty if there is no such unit.
    #[func]
    fn made(&self, id: GString, faction: GString, direction: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        let Some(content) = self.content.as_ref() else {
            return out;
        };
        let Some(unit) = content.units.get(&id.to_string()) else {
            return out;
        };
        let mut lexicon = words::Lexicon::default();
        let made = made::unit(content, unit, &faction.to_string(), direction, &mut lexicon);
        let sheet: PackedFloat64Array =
            (0..sheet::NAMES.len()).map(|n| made.sheet.get(n)).collect();
        out.set("sheet", &sheet.to_variant());
        out.set("words", &said(&lexicon, &made.words).to_variant());
        out.set("walker", &PackedFloat64Array::from(made.walker.as_slice()).to_variant());
        out
    }

    /// A fresh definition of each kind filled with the crate's defaults, for the test that
    /// holds them equal to GDScript's.
    #[func]
    fn defaults() -> VarDictionary {
        let mut out = VarDictionary::new();
        let mut put = |class: &str, fields: serde_json::Map<String, serde_json::Value>| {
            out.set(class, &made_from(class, &fields).to_variant());
        };
        let mut unit = json_fields(&content::Unit::default());
        unit.remove("items");
        put("UnitDef", unit);
        put("WeaponDef", json_fields(&content::Weapon::default()));
        put("ArmourDef", json_fields(&content::Armour::default()));
        put("TerrainLibraryDef", json_fields(&content::TerrainLibrary::default()));
        put("TerrainDef", json_fields(&content::Terrain::default()));
        put("LiquidBodyDef", json_fields(&content::LiquidBody::default()));
        put("StratumDef", json_fields(&content::Stratum::default()));
        put("TerrainFeatureDef", json_fields(&content::TerrainFeature::default()));
        put("MaterialDef", json_fields(&content::Material::default()));
        put("HeatTransitionDef", json_fields(&content::HeatTransition::default()));
        put("NodeDef", json_fields(&content::Node::default()));
        put("GarrisonUnitDef", json_fields(&content::GarrisonUnit::default()));
        put("SubnodeDef", json_fields(&content::Subnode::default()));
        put("StructurePlanDef", json_fields(&content::StructurePlan::default()));
        put("StructureFaceDef", json_fields(&content::StructureFace::default()));
        put("MapEdgeDef", json_fields(&content::MapEdge::default()));
        put("FactionDef", json_fields(&content::Faction::default()));
        put("FactionRelationDef", json_fields(&content::FactionRelation::default()));
        put("LossGroupDef", json_fields(&content::LossGroup::default()));
        put("MapLayoutDef", json_fields(&content::MapLayout::default()));
        put("TileDef", json_fields(&content::Tile::default()));
        put("BridgeDef", json_fields(&content::Bridge::default()));
        put("RoadSegmentDef", json_fields(&content::RoadSegment::default()));
        put("RouteDef", json_fields(&content::Route::default()));
        put("ChannelDef", json_fields(&content::Channel::default()));
        let mut lane = json_fields(&content::Lane::default());
        lane.remove("nodes");
        put("LaneDef", lane);
        out.set("MapDef", &map_object(&content::Map::default()).to_variant());
        out
    }

    /// The designer's `export` (a saved designer map) as a MapDef, read and checked as the
    /// designer's save reads it, its terrain checked against the loaded library: {"map",
    /// "errors", "warnings"}. What the tests that build maps from designer exports use.
    #[func]
    fn map_from_export(&self, export: VarDictionary) -> VarDictionary {
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        let mut read = designer::map_from_export(&json_of(&export.to_variant()), &library);
        if read.errors.is_empty() {
            read.errors = read.map.validate(&library);
        }
        let mut out = VarDictionary::new();
        out.set("map", &map_object(&read.map).to_variant());
        out.set("errors", &variant_of(&read.errors));
        out.set("warnings", &variant_of(&read.warnings));
        out
    }

    /// A designer structure plan (spec 24) as a StructurePlanDef.
    #[func]
    fn plan_from_export(drawn: VarDictionary) -> Variant {
        let (plan, errors) = designer::plan_from_export(&json_of(&drawn.to_variant()));
        for error in errors {
            godot_error!("ContentStore: {error}");
        }
        made_from("StructurePlanDef", &json_fields(&plan)).to_variant()
    }

    /// The map file at `path` (an OS path), read fresh and checked against the loaded
    /// library: {"map": its MapDef}, or {"error": why it didn't load}.
    #[func]
    fn read_map(&self, path: GString) -> VarDictionary {
        let mut out = VarDictionary::new();
        let path = std::path::Path::new(&path.to_string()).to_path_buf();
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        match content::load_file::<content::Map>(&path) {
            Ok(map) => match map.check(&library) {
                Ok(()) => out.set("map", &map_object(&map).to_variant()),
                Err(why) => out.set("error", &format!("{}: {why}", path.display()).to_variant()),
            },
            Err(error) => out.set("error", &error.to_string().to_variant()),
        }
        out
    }

    /// Writes `map` (a MapDef) to `path` (an OS path) as a map file, once it fits the schema
    /// and keeps the map's rules against the loaded library: "" when written, else why not.
    #[func]
    fn write_map(&self, map: Gd<Object>, path: GString) -> GString {
        let library = self.content.as_ref().map(|c| c.terrain.clone()).unwrap_or_default();
        let value = map_fields(&map);
        let read: Result<content::Map, _> = serde_path_to_error::deserialize(value);
        let written = read
            .map_err(|error| format!("{}: {}", error.path(), error.inner()))
            .and_then(|map| map.check(&library).map(|()| map))
            .and_then(|map| {
                std::fs::write(path.to_string(), content::file_text(&map))
                    .map_err(|error| format!("{path}: {error}"))
            });
        match written {
            Ok(()) => GString::new(),
            Err(why) => GString::from(&why),
        }
    }
}

impl ContentStore {
    /// The item at `path` (weapons/<id> or armour/<id>), made once; null if there is none.
    fn item(&mut self, path: &str) -> Variant {
        if let Some(made) = self.made.get(path) {
            return made.to_variant();
        }
        let content = self.content.as_ref();
        let (class, fields) = match path.split_once('/') {
            Some(("weapons", id)) => {
                ("WeaponDef", content.and_then(|c| c.weapons.get(id)).map(json_fields))
            }
            Some(("armour", id)) => {
                ("ArmourDef", content.and_then(|c| c.armour.get(id)).map(json_fields))
            }
            _ => ("", None),
        };
        self.made_once(path, class, fields)
    }

    fn made_once(
        &mut self,
        key: &str,
        class: &str,
        fields: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Variant {
        if let Some(made) = self.made.get(key) {
            return made.to_variant();
        }
        let Some(fields) = fields else {
            return Variant::nil();
        };
        let made = made_from(class, &fields);
        self.made.insert(key.to_string(), made.clone());
        made.to_variant()
    }
}

/// `value`'s fields as JSON.
fn json_fields(value: &impl serde::Serialize) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(fields)) => fields,
        _ => serde_json::Map::new(),
    }
}

/// A MapDef: every node made once (a node on two lanes is one NodeDef), lanes holding them in
/// order, the off-lane rest, and the other fields by name.
fn map_object(map: &content::Map) -> Gd<Object> {
    let mut fields = json_fields(map);
    fields.remove("nodes");
    fields.remove("lanes");
    let nodes: Vec<(String, Gd<Object>)> = map
        .nodes
        .iter()
        .map(|node| (node.id.clone(), made_from("NodeDef", &json_fields(node))))
        .collect();
    let node = |id: &str| nodes.iter().find(|(known, _)| known == id).map(|(_, made)| made.clone());
    let made = made_from("MapDef", &fields);
    let mut lanes = VarArray::new();
    for lane in &map.lanes {
        let mut lane_fields = json_fields(lane);
        lane_fields.remove("nodes");
        let made_lane = made_from("LaneDef", &lane_fields);
        let on_lane: VarArray =
            lane.nodes.iter().filter_map(|id| node(id)).map(|n| n.to_variant()).collect();
        made_lane.get("nodes").call("assign", &[on_lane.to_variant()]);
        lanes.push(&made_lane.to_variant());
    }
    made.get("lanes").call("assign", &[lanes.to_variant()]);
    let off_lane: VarArray =
        map.off_lane_nodes().filter_map(|n| node(&n.id)).map(|n| n.to_variant()).collect();
    made.get("off_lane_nodes").call("assign", &[off_lane.to_variant()]);
    made
}

/// A new definition of the GDScript class `class` (a `class_name`), filled from `fields`.
fn made_from(class: &str, fields: &serde_json::Map<String, serde_json::Value>) -> Gd<Object> {
    let mut made = new_of(&script_named(class));
    fill(&mut made, fields);
    made
}

/// The script declaring `class_name class`; nil if there is none.
fn script_named(class: &str) -> Variant {
    let classes = godot::classes::ProjectSettings::singleton().get_global_class_list();
    for entry in classes.iter_shared() {
        if entry.get("class").is_some_and(|name| name.to_string() == class) {
            let path = entry.get("path").map(|path| path.to_string()).unwrap_or_default();
            return godot::tools::try_load::<godot::classes::Script>(&path)
                .map(|script| script.to_variant())
                .unwrap_or_default();
        }
    }
    godot_error!("ContentStore: no GDScript class {class}");
    Variant::nil()
}

/// An instance of `script` (a Script variant).
fn new_of(script: &Variant) -> Gd<Object> {
    script.call("new", &[]).to::<Gd<Object>>()
}

/// Each property `made` saves (its script's exported fields), by name.
fn saved_properties(made: &Gd<Object>) -> Vec<VarDictionary> {
    const SCRIPT_VARIABLE: i64 = 4096;
    const STORAGE: i64 = 2;
    made.get_property_list()
        .iter_shared()
        .filter(|property| {
            let usage = property.get("usage").map_or(0, |usage| usage.to::<i64>());
            usage & SCRIPT_VARIABLE != 0 && usage & STORAGE != 0
        })
        .collect()
}

fn property_named(made: &Gd<Object>, key: &str) -> Option<VarDictionary> {
    made.get_property_list()
        .iter_shared()
        .find(|property| property.get("name").is_some_and(|name| name.to_string() == key))
}

/// Sets each of `fields` on `made` by name. An item's shared fields ("item") are its own; a
/// typed list keeps its type ("inf" in a list of reals is an infinity, and a list of
/// definitions is made one by one); a name where an enum is kept is the enum's value; a
/// colour is its "#rrggbb"; a vector its list of numbers; a dictionary keyed by cells its
/// [cell, value] pairs; and a definition held alone is made from its fields (none for null).
fn fill(made: &mut Gd<Object>, fields: &serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;
    for (key, value) in fields {
        if key == "item" {
            if let Value::Object(shared) = value {
                fill(made, shared);
            }
            continue;
        }
        let current = made.get(key.as_str());
        let set = match (current.get_type(), value) {
            (VariantType::ARRAY, Value::Array(values)) => {
                let typed = current.call("get_typed_builtin", &[]).to::<i64>();
                let script = current.call("get_typed_script", &[]);
                let list: VarArray =
                    values.iter().map(|item| element(item, typed, &script)).collect();
                current.call("assign", &[list.to_variant()]);
                continue;
            }
            (VariantType::DICTIONARY, Value::Array(pairs)) => cell_keyed(pairs),
            (
                VariantType::VECTOR2 | VariantType::VECTOR2I | VariantType::VECTOR3I,
                Value::Array(numbers),
            ) => vector(numbers, current.get_type()),
            (VariantType::COLOR, Value::String(text)) => {
                Color::from_html(text.as_str()).unwrap_or(Color::WHITE).to_variant()
            }
            (VariantType::INT, Value::String(name)) => enum_value(made, key, name).to_variant(),
            (VariantType::NIL, Value::Object(inner)) => {
                let class = property_named(made, key)
                    .and_then(|property| property.get("hint_string"))
                    .map(|hint| hint.to_string())
                    .unwrap_or_default();
                made_from(&class, inner).to_variant()
            }
            _ => json_variant(value),
        };
        made.set(key.as_str(), &set);
    }
}

/// A list element as its list's type (`typed`, a Variant.Type, or a definition of `script`).
fn element(item: &serde_json::Value, typed: i64, script: &Variant) -> Variant {
    use serde_json::Value;
    let typed = VariantType::from_ord(typed as i32);
    match (typed, item) {
        (VariantType::FLOAT, Value::String(text)) => {
            (if text == "-inf" { f64::NEG_INFINITY } else { f64::INFINITY }).to_variant()
        }
        (
            VariantType::VECTOR2 | VariantType::VECTOR2I | VariantType::VECTOR3I,
            Value::Array(numbers),
        ) => vector(numbers, typed),
        (VariantType::OBJECT, Value::Object(fields)) => {
            let mut made = new_of(script);
            fill(&mut made, fields);
            made.to_variant()
        }
        _ => json_variant(item),
    }
}

/// [x, y] or [x, y, level] as a vector of `typed` (a point, a cell, a structure cell).
fn vector(numbers: &[serde_json::Value], typed: VariantType) -> Variant {
    let at = |index: usize| numbers.get(index).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    match typed {
        VariantType::VECTOR2 => Vector2::new(at(0) as f32, at(1) as f32).to_variant(),
        VariantType::VECTOR3I => {
            Vector3i::new(at(0) as i32, at(1) as i32, at(2) as i32).to_variant()
        }
        _ => Vector2i::new(at(0) as i32, at(1) as i32).to_variant(),
    }
}

/// [[cell, value], ...] as {cell: value}, in order; a cell of three numbers is a structure cell.
fn cell_keyed(pairs: &[serde_json::Value]) -> Variant {
    let mut out = VarDictionary::new();
    for pair in pairs.iter().filter_map(serde_json::Value::as_array) {
        let (Some(serde_json::Value::Array(cell)), Some(value)) = (pair.first(), pair.get(1))
        else {
            continue;
        };
        let typed = if cell.len() == 3 { VariantType::VECTOR3I } else { VariantType::VECTOR2I };
        out.set(&vector(cell, typed), &json_variant(value));
    }
    out.to_variant()
}

/// The value of `made`'s enum `key` named `name` (case aside), from its export's hint:
/// "Origin:0,Resource:1" for a named enum, "north,west,floor" for an export_enum.
fn enum_value(made: &Gd<Object>, key: &str, name: &str) -> i64 {
    let hint = property_named(made, key)
        .and_then(|p| p.get("hint_string"))
        .map(|h| h.to_string())
        .unwrap_or_default();
    let wanted = name.to_lowercase().replace('_', " ");
    for (index, entry) in hint.split(',').enumerate() {
        let (label, at) = entry
            .split_once(':')
            .map_or((entry, index as i64), |(label, at)| (label, at.parse().unwrap_or(0)));
        if label.to_lowercase().replace('_', " ") == wanted {
            return at;
        }
    }
    godot_error!("ContentStore: {name} isn't one of {key}'s values");
    0
}

/// The name of `made`'s enum value `value` in the schema: a named enum's in capitals
/// (Godot writes ORE_VEIN as "Ore Vein"), an export_enum's as written.
fn enum_name(property: &VarDictionary, value: i64) -> String {
    let hint = property.get("hint_string").map(|h| h.to_string()).unwrap_or_default();
    for (index, entry) in hint.split(',').enumerate() {
        match entry.split_once(':') {
            Some((label, at)) if at.parse::<i64>().ok() == Some(value) => {
                return label.to_uppercase().replace(' ', "_");
            }
            None if index as i64 == value => return entry.to_string(),
            _ => {}
        }
    }
    value.to_string()
}

/// A MapDef's fields in the schema: every node once (those on lanes first) under "nodes",
/// and each lane naming its nodes by id.
fn map_fields(map: &Gd<Object>) -> serde_json::Value {
    use serde_json::Value;
    let lanes: Vec<Gd<Object>> = objects(&map.get("lanes"));
    let mut nodes: Vec<Gd<Object>> = Vec::new();
    let on_lanes = lanes.iter().flat_map(|lane| objects(&lane.get("nodes")));
    for node in on_lanes.chain(objects(&map.get("off_lane_nodes"))) {
        if !nodes.iter().any(|known| known.instance_id() == node.instance_id()) {
            nodes.push(node);
        }
    }
    let mut fields = object_fields(map);
    fields.remove("off_lane_nodes");
    fields.insert(
        "nodes".into(),
        Value::Array(nodes.iter().map(|n| Value::Object(object_fields(n))).collect()),
    );
    let lanes = lanes.iter().map(|lane| {
        let ids = objects(&lane.get("nodes"))
            .iter()
            .map(|n| Value::String(n.get("id").to_string()))
            .collect();
        let mut lane_fields = object_fields(lane);
        lane_fields.insert("nodes".into(), Value::Array(ids));
        Value::Object(lane_fields)
    });
    fields.insert("lanes".into(), Value::Array(lanes.collect()));
    Value::Object(fields)
}

fn objects(list: &Variant) -> Vec<Gd<Object>> {
    items(list).iter().filter_map(|item| item.try_to::<Gd<Object>>().ok()).collect()
}

/// The elements of a list, typed (`Array[NodeDef]`) or not: Rust can't hold a typed GDScript
/// array as an untyped one, so they are read one by one.
fn items(list: &Variant) -> Vec<Variant> {
    let size = list.call("size", &[]).try_to::<i64>().unwrap_or(0);
    (0..size).map(|index| list.call("get", &[index.to_variant()])).collect()
}

/// A definition's saved fields by name, in the schema.
fn object_fields(made: &Gd<Object>) -> serde_json::Map<String, serde_json::Value> {
    saved_properties(made)
        .iter()
        .map(|property| {
            let name = property.get("name").map(|name| name.to_string()).unwrap_or_default();
            let value = schema_value(&made.get(name.as_str()), Some(property));
            (name, value)
        })
        .collect()
}

/// A field's value in the schema: a definition its fields, a vector its numbers, a cell-keyed
/// dictionary its [cell, value] pairs, an enum its name.
fn schema_value(value: &Variant, property: Option<&VarDictionary>) -> serde_json::Value {
    use serde_json::Value;
    const HINT_ENUM: i64 = 2;
    match value.get_type() {
        VariantType::OBJECT => match value.try_to::<Gd<Object>>() {
            Ok(made) => Value::Object(object_fields(&made)),
            Err(_) => Value::Null,
        },
        VariantType::ARRAY => {
            Value::Array(items(value).iter().map(|item| schema_value(item, None)).collect())
        }
        VariantType::DICTIONARY => Value::Array(
            value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, item)| {
                    Value::Array(vec![schema_value(&key, None), schema_value(&item, None)])
                })
                .collect(),
        ),
        VariantType::VECTOR2 => {
            let at = value.to::<Vector2>();
            serde_json::json!([at.x as f64, at.y as f64])
        }
        VariantType::VECTOR2I => {
            let at = value.to::<Vector2i>();
            serde_json::json!([at.x, at.y])
        }
        VariantType::VECTOR3I => {
            let at = value.to::<Vector3i>();
            serde_json::json!([at.x, at.y, at.z])
        }
        VariantType::INT => {
            match property.filter(|p| p.get("hint").is_some_and(|h| h.to::<i64>() == HINT_ENUM)) {
                Some(property) => Value::String(enum_name(property, value.to::<i64>())),
                None => Value::from(value.to::<i64>()),
            }
        }
        _ => json_of(value),
    }
}

/// Godot data as JSON: a whole number stays one, a real stays one.
fn json_of(value: &Variant) -> serde_json::Value {
    use serde_json::Value;
    match value.get_type() {
        VariantType::BOOL => Value::Bool(value.to::<bool>()),
        VariantType::INT => Value::from(value.to::<i64>()),
        VariantType::FLOAT => Value::from(value.to::<f64>()),
        VariantType::STRING | VariantType::STRING_NAME => Value::String(value.to_string()),
        VariantType::ARRAY => Value::Array(items(value).iter().map(json_of).collect()),
        VariantType::PACKED_STRING_ARRAY => Value::Array(
            value
                .to::<PackedStringArray>()
                .as_slice()
                .iter()
                .map(|text| Value::String(text.to_string()))
                .collect(),
        ),
        VariantType::DICTIONARY => Value::Object(
            value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, item)| (key.to_string(), json_of(&item)))
                .collect(),
        ),
        _ => Value::Null,
    }
}

fn variant_of(value: &impl serde::Serialize) -> Variant {
    json_variant(&serde_json::to_value(value).expect("content serialises"))
}

fn json_variant(value: &serde_json::Value) -> Variant {
    use serde_json::Value;
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(flag) => flag.to_variant(),
        Value::Number(number) => match number.as_i64() {
            Some(whole) => whole.to_variant(),
            None => number.as_f64().unwrap_or(f64::NAN).to_variant(),
        },
        Value::String(text) => text.to_variant(),
        Value::Array(items) => {
            let mut array = VarArray::new();
            for item in items {
                array.push(&json_variant(item));
            }
            array.to_variant()
        }
        Value::Object(fields) => {
            let mut dictionary = VarDictionary::new();
            for (key, item) in fields {
                dictionary.set(&key.to_variant(), &json_variant(item));
            }
            dictionary.to_variant()
        }
    }
}

/// `of`'s words and lists by name, decoded through `lexicon`, as SkirmishUnit holds them.
fn said(lexicon: &words::Lexicon, of: &words::Words) -> VarDictionary {
    let mut held = of.clone();
    let mut out = VarDictionary::new();
    for name in words::WORDS {
        let code = *held.word(name).unwrap();
        out.set(*name, &GString::from(lexicon.word(code)).to_variant());
    }
    for name in words::LISTS {
        let list: PackedStringArray =
            held.list(name).unwrap().iter().map(|&c| GString::from(lexicon.word(c))).collect();
        out.set(*name, &list.to_variant());
    }
    for name in words::LEVELS {
        let mut levels = VarDictionary::new();
        for &(code, level) in held.levels(name).unwrap().iter() {
            levels.set(lexicon.word(code), level);
        }
        out.set(*name, &levels.to_variant());
    }
    for name in words::PARTS {
        let mut parts = VarArray::new();
        for part in held.parts(name).unwrap().iter() {
            let kind = GString::from(lexicon.word(part.kind));
            parts.push(&varray![part.damage, &kind, part.magical].to_variant());
        }
        out.set(*name, &parts.to_variant());
    }
    out
}
