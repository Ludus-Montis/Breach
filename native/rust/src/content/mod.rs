//! The simulation's content (Decision 153, spec 39 step 2): the crate's serde structs are its
//! schema, and the files under `content/` are JSON in it. `units/`, `weapons/`, `armour/`
//! and `tuning/battle_tuning.json`, each file's id its name without `.json`, the terrain
//! library, `terrain/terrain_library.json`, and the maps, `maps/<name>.json`. A file that
//! doesn't match fails to load, naming the file and the field. `schema.rs` writes the JSON
//! Schema the designer validates against (`content/schema/`).

mod items;
mod map;
mod map_check;
#[cfg(test)]
mod map_check_tests;
mod reals;
#[cfg(test)]
mod schema;
mod terrain;
mod tuning;
mod unit;
mod write;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

pub use items::{Armour, Weapon};
pub use map::{
    Bridge, Channel, Faction, FactionRelation, GarrisonUnit, Lane, LossGroup, LossRule, Map,
    MapEdge, MapLayout, Node, NodeType, ResourceType, RoadSegment, Route, Side, Stance,
    StructureFace, StructurePlan, Subnode, Tile,
};
pub use map_check::{cell, CELLS_PER_TILE};
pub use terrain::{
    Colour, HeatTransition, LiquidBody, Material, Stratum, Terrain, TerrainFeature, TerrainLibrary,
};
pub use tuning::BattleTuning;
pub use unit::Unit;
pub use write::file_text;

/// The terrain library's file under `content/`.
const TERRAIN_LIBRARY: &str = "terrain/terrain_library.json";

/// Why content didn't load: the file, the field (`items[1]`, `item.slots`; `.` for the
/// whole file) and what was wrong.
#[derive(Debug)]
pub struct LoadError {
    pub file: PathBuf,
    pub field: String,
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}: {}: {}", self.file.display(), self.field, self.message)
    }
}

/// Everything the simulation reads from `content/`, by id in id order.
#[derive(Debug, Serialize)]
pub struct Content {
    pub units: BTreeMap<String, Unit>,
    pub weapons: BTreeMap<String, Weapon>,
    pub armour: BTreeMap<String, Armour>,
    pub tuning: BattleTuning,
    pub terrain: TerrainLibrary,
    pub maps: BTreeMap<String, Map>,
}

impl Content {
    /// Loads the content under `root` (the repo's `content/`), every unit's items, every
    /// material the terrain library names, every node, terrain and feature a map names, and
    /// each map's rules (`map_check.rs`) checked.
    pub fn load(root: &Path) -> Result<Content, LoadError> {
        let content = Content {
            units: load_dir(&root.join("units"))?,
            weapons: load_dir(&root.join("weapons"))?,
            armour: load_dir(&root.join("armour"))?,
            tuning: load_file(&root.join("tuning").join("battle_tuning.json"))?,
            terrain: load_file(&root.join(TERRAIN_LIBRARY))?,
            maps: load_dir(&root.join("maps"))?,
        };
        content.check_items(root)?;
        if let Some((field, id)) = content.terrain.missing_material() {
            let message = format!("no material {id}");
            return Err(LoadError { file: root.join(TERRAIN_LIBRARY), field, message });
        }
        for (name, map) in &content.maps {
            let file = root.join("maps").join(format!("{name}.json"));
            if let Some((field, id)) = map.missing_reference(&content.terrain) {
                return Err(LoadError { file, field, message: format!("no {id} here") });
            }
            let broken = map.validate(&content.terrain);
            if !broken.is_empty() {
                return Err(LoadError { file, field: ".".into(), message: broken.join("; ") });
            }
        }
        Ok(content)
    }

    /// Fails on the first item a unit names that isn't here.
    fn check_items(&self, root: &Path) -> Result<(), LoadError> {
        for (id, unit) in &self.units {
            for (index, item) in unit.items.iter().enumerate() {
                if !self.has_item(item) {
                    return Err(LoadError {
                        file: root.join("units").join(format!("{id}.json")),
                        field: format!("items[{index}]"),
                        message: format!("no item {item} (weapons/<id> or armour/<id>)"),
                    });
                }
            }
        }
        Ok(())
    }

    /// Whether `path` (`weapons/<id>` or `armour/<id>`) names an item here.
    pub fn has_item(&self, path: &str) -> bool {
        match path.split_once('/') {
            Some(("weapons", id)) => self.weapons.contains_key(id),
            Some(("armour", id)) => self.armour.contains_key(id),
            _ => false,
        }
    }
}

/// Every `.json` file in `dir` by id, in id order; none if `dir` isn't there.
fn load_dir<T: DeserializeOwned>(dir: &Path) -> Result<BTreeMap<String, T>, LoadError> {
    let mut loaded = BTreeMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(loaded);
    };
    let mut paths: Vec<PathBuf> =
        entries.filter_map(|entry| entry.ok().map(|e| e.path())).collect();
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "json"));
    paths.sort();
    for path in paths {
        let id = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        loaded.insert(id, load_file(&path)?);
    }
    Ok(loaded)
}

/// One file, read as `T`.
pub fn load_file<T: DeserializeOwned>(path: &Path) -> Result<T, LoadError> {
    let failed = |field: String, message: String| LoadError { file: path.into(), field, message };
    let text = fs::read_to_string(path).map_err(|error| failed(".".into(), error.to_string()))?;
    let mut json = serde_json::Deserializer::from_str(&text);
    let read = serde_path_to_error::deserialize(&mut json)
        .map_err(|error| failed(error.path().to_string(), error.inner().to_string()))?;
    json.end().map_err(|error| failed(".".into(), error.to_string()))?;
    Ok(read)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A fresh directory under the system's temp dir, for files a test writes.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("breach-content-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_field_of_the_wrong_type_fails_naming_the_file_and_the_field() {
        let dir = scratch("wrong-type");
        let path = dir.join("grem.json");
        fs::write(&path, r#"{"hp": 20, "slots": ["hand", 3]}"#).unwrap();
        let error = load_file::<Unit>(&path).unwrap_err();
        assert_eq!(error.file, path);
        assert_eq!(error.field, "slots[1]");
        assert!(error.to_string().starts_with(&format!("{}: slots[1]: ", path.display())));
    }

    #[test]
    fn a_field_no_unit_has_fails_naming_it() {
        let dir = scratch("unknown-field");
        let path = dir.join("grem.json");
        fs::write(&path, r#"{"hp": 20, "hit_points": 20}"#).unwrap();
        let error = load_file::<Unit>(&path).unwrap_err();
        assert!(error.message.contains("hit_points"), "{error}");
    }

    #[test]
    fn a_unit_naming_an_item_that_isnt_there_fails_naming_the_item() {
        let root = scratch("missing-item");
        fs::create_dir_all(root.join("units")).unwrap();
        fs::create_dir_all(root.join("weapons")).unwrap();
        copy_required(&root, &["tuning/battle_tuning.json", TERRAIN_LIBRARY]);
        fs::write(root.join("weapons/claw.json"), r#"{"damage": 3}"#).unwrap();
        fs::write(root.join("units/grem.json"), r#"{"items": ["weapons/claw", "weapons/bite"]}"#)
            .unwrap();
        let error = Content::load(&root).unwrap_err();
        assert_eq!(error.file, root.join("units/grem.json"));
        assert_eq!(error.field, "items[1]");
    }

    #[test]
    fn a_file_that_omits_a_field_takes_gdscripts_default() {
        let dir = scratch("defaults");
        let path = dir.join("spear.json");
        fs::write(&path, r#"{"item": {"item_name": "spear"}, "damage": 5}"#).unwrap();
        let weapon = load_file::<Weapon>(&path).unwrap();
        assert_eq!(weapon.item.item_name, "spear");
        assert_eq!(weapon.damage_floor, 0.5);
        assert_eq!(weapon.attack_seconds, 1.0);
    }

    #[test]
    fn the_tuning_names_every_field() {
        let dir = scratch("tuning-missing");
        let path = dir.join("battle_tuning.json");
        fs::write(&path, r#"{"morale_loss": 4}"#).unwrap();
        let error = load_file::<BattleTuning>(&path).unwrap_err();
        assert!(error.message.contains("missing field"), "{error}");
    }

    #[test]
    fn a_terrain_field_of_the_wrong_type_fails_naming_it() {
        let dir = scratch("terrain-wrong-type");
        let path = dir.join("terrain_library.json");
        fs::write(&path, r#"{"terrains": [{"id": "FIELDS"}, {"id": "BOG", "move_cost": "slow"}]}"#)
            .unwrap();
        let error = load_file::<TerrainLibrary>(&path).unwrap_err();
        assert_eq!(error.field, "terrains[1].move_cost");
    }

    #[test]
    fn a_colour_that_isnt_hex_fails_naming_it() {
        let dir = scratch("terrain-colour");
        let path = dir.join("terrain_library.json");
        fs::write(&path, r#"{"materials": [{"id": "ROCK", "color": "grey"}]}"#).unwrap();
        let error = load_file::<TerrainLibrary>(&path).unwrap_err();
        assert_eq!(error.field, "materials[0].color");
    }

    #[test]
    fn a_stratum_of_a_material_that_isnt_there_fails_naming_it() {
        let root = scratch("terrain-missing-material");
        copy_required(&root, &["tuning/battle_tuning.json"]);
        fs::create_dir_all(root.join("terrain")).unwrap();
        let library = r#"{"materials": [{"id": "SOIL"}], "terrains": [{"id": "FIELDS",
            "strata": [{"material_id": "SOIL"}, {"material_id": "CLAY"}]}]}"#;
        fs::write(root.join("terrain/terrain_library.json"), library).unwrap();
        let error = Content::load(&root).unwrap_err();
        assert_eq!(error.file, root.join("terrain/terrain_library.json"));
        assert_eq!(error.field, "terrains[0].strata[1].material_id");
    }

    #[test]
    fn a_terrain_that_omits_a_field_takes_gdscripts_default() {
        let dir = scratch("terrain-defaults");
        let path = dir.join("terrain_library.json");
        let text = r#"{"terrains": [{"id": "FIELDS", "strata": [{"material_id": "SOIL"}]}],
            "materials": [{"id": "SOIL"}]}"#;
        fs::write(&path, text).unwrap();
        let library = load_file::<TerrainLibrary>(&path).unwrap();
        let fields = &library.terrains[0];
        assert!(fields.can_be_base);
        assert_eq!(fields.move_cost, 1.0);
        assert_eq!(fields.color.as_str(), "#ffffff");
        assert_eq!((fields.strata[0].min_cells, fields.strata[0].max_cells), (1, 1));
        assert_eq!((library.materials[0].weight, library.materials[0].heat), (1, 1));
        assert_eq!(library.road_move_multiplier, 0.5);
    }

    #[test]
    fn the_checked_in_content_loads() {
        let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
        assert!(content.units.contains_key("grem"));
        assert_eq!(content.tuning.discipline_leash_steps.last(), Some(&f64::INFINITY));
        let fields = content.terrain.terrains.iter().find(|terrain| terrain.id == "FIELDS");
        assert!(fields.is_some_and(|fields| fields.can_be_base));
    }

    #[test]
    fn a_map_field_of_the_wrong_type_fails_naming_it() {
        let dir = scratch("map-wrong-type");
        let path = dir.join("m.json");
        fs::write(&path, r#"{"nodes": [{"id": "p"}, {"id": "f", "tile": [1, "2"]}]}"#).unwrap();
        let error = load_file::<Map>(&path).unwrap_err();
        assert_eq!(error.field, "nodes[1].tile[1]");
    }

    #[test]
    fn a_node_type_that_isnt_one_fails_naming_it() {
        let dir = scratch("map-node-type");
        let path = dir.join("m.json");
        fs::write(&path, r#"{"nodes": [{"id": "p", "node_type": "CASTLE"}]}"#).unwrap();
        let error = load_file::<Map>(&path).unwrap_err();
        assert_eq!(error.field, "nodes[0].node_type");
    }

    #[test]
    fn a_lane_naming_a_node_that_isnt_there_fails_naming_it() {
        let root = scratch("map-missing-node");
        copy_required(&root, &["tuning/battle_tuning.json", TERRAIN_LIBRARY]);
        fs::create_dir_all(root.join("maps")).unwrap();
        let map = r#"{"nodes": [{"id": "p"}, {"id": "c"}],
            "lanes": [{"id": "to_c", "nodes": ["p", "f", "c"]}]}"#;
        fs::write(root.join("maps/m.json"), map).unwrap();
        let error = Content::load(&root).unwrap_err();
        assert_eq!(error.file, root.join("maps/m.json"));
        assert_eq!(error.field, "lanes[0].nodes[1]");
    }

    #[test]
    fn a_tile_of_a_terrain_that_isnt_there_fails_naming_it() {
        let root = scratch("map-missing-terrain");
        copy_required(&root, &["tuning/battle_tuning.json", TERRAIN_LIBRARY]);
        fs::create_dir_all(root.join("maps")).unwrap();
        let map = r#"{"layout": {"cols": 2, "rows": 1, "default_terrain_id": "FIELDS",
            "tiles": [{"cell": [0, 0], "terrain_id": "FOREST"}, {"cell": [1, 0],
            "terrain_id": "LAVA_SEA"}]}}"#;
        fs::write(root.join("maps/m.json"), map).unwrap();
        let error = Content::load(&root).unwrap_err();
        assert_eq!(error.field, "layout.tiles[1].terrain_id");
    }

    #[test]
    fn a_map_that_breaks_a_rule_fails_to_load_naming_it() {
        let root = scratch("map-rule");
        copy_required(&root, &["tuning/battle_tuning.json", TERRAIN_LIBRARY]);
        fs::create_dir_all(root.join("maps")).unwrap();
        let map = r#"{"nodes": [{"id": "p"}, {"id": "c", "garrison": -1}],
            "lanes": [{"id": "to_c", "nodes": ["p", "c"]}], "tick_duration_seconds": 1.5}"#;
        fs::write(root.join("maps/m.json"), map).unwrap();
        let error = Content::load(&root).unwrap_err();
        assert_eq!(error.file, root.join("maps/m.json"));
        assert!(error.message.contains("garrison must be >= 0, got -1"), "{error}");
    }

    #[test]
    fn a_map_that_omits_a_field_takes_gdscripts_default() {
        let dir = scratch("map-defaults");
        let path = dir.join("m.json");
        let text = r#"{"nodes": [{"id": "p", "plan": {"faces": [{}]}}],
            "layout": {"tiles": [{"bridge": {}}], "channels": [{}]}}"#;
        fs::write(&path, text).unwrap();
        let map = load_file::<Map>(&path).unwrap();
        let node = &map.nodes[0];
        assert!(node.is_inexhaustible);
        assert_eq!(node.tile, [-1, -1]);
        assert_eq!(node.plan.as_ref().unwrap().faces[0].thickness, 1);
        let layout = map.layout.unwrap();
        assert_eq!((layout.cell_size, layout.ceiling), (64, 64));
        assert_eq!(layout.tiles[0].elevation, -1);
        assert_eq!(layout.tiles[0].bridge.as_ref().unwrap().hp, 40);
        assert_eq!((layout.channels[0].width, layout.channels[0].depth), (4, 2));
    }

    #[test]
    fn the_checked_in_maps_load() {
        let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
        for name in ["demo_map", "p_f_F_c", "skirmish_p_c", "skirmish_two_lanes"] {
            assert!(content.maps.get(name).is_some_and(|map| !map.lanes.is_empty()), "{name}");
        }
    }

    /// Copies each of `files` from the repo's `content/` to the same place under `root`.
    fn copy_required(root: &Path, files: &[&str]) {
        for file in files {
            let to = root.join(file);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(repo_content().join(file), to).unwrap();
        }
    }

    /// The repo's `content/`.
    pub fn repo_content() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
    }
}
