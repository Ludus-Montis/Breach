//! The designer's map export read into the schema (specs 16, 19, 24, 26: the scenarios of
//! Godot's designer import), the terrain library both ways (spec 39 step 2b), and the round
//! trip: each saved designer map, read, equals the checked-in map.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::terrain::{from_schema, in_use_problems, to_schema, used_ids};
use super::*;
use crate::content::{load_file, Content, Node, NodeType, Side};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn library() -> TerrainLibrary {
    load_file(&repo().join("content/terrain/terrain_library.json")).unwrap()
}

fn fixture(name: &str) -> Value {
    let path = repo().join("tests/fixtures/designer").join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// The designer's demo export (links, fields, garrisons, factions).
fn demo() -> Value {
    fixture("demo_map.designer.json")
}

/// A designer export with a layout (tiles, roads, routes, loss groups).
fn layout_map() -> Value {
    fixture("layout_map.designer.json")
}

fn read(export: &Value) -> Translated {
    map_from_export(export, &library())
}

fn node_entry<'a>(export: &'a mut Value, id: &str) -> &'a mut Value {
    let nodes = export["nodes"].as_array_mut().unwrap();
    nodes.iter_mut().find(|node| node["id"] == id).unwrap()
}

fn node<'a>(map: &'a Map, id: &str) -> &'a Node {
    map.nodes.iter().find(|node| node.id == id).unwrap()
}

fn any(messages: &[String], needle: &str) -> bool {
    messages.iter().any(|message| message.contains(needle))
}

/// The off-lane nodes' ids, in order.
fn off_lane(map: &Map) -> Vec<&str> {
    let on_lane = |id: &str| map.lanes.iter().any(|lane| lane.nodes.iter().any(|on| on == id));
    map.nodes.iter().map(|node| node.id.as_str()).filter(|id| !on_lane(id)).collect()
}

#[test]
fn the_demo_map_derives_one_lane_from_its_links() {
    let read = read(&demo());
    assert!(read.errors.is_empty(), "{:?}", read.errors);
    assert_eq!(read.map.lanes.len(), 1);
    let lane = &read.map.lanes[0];
    assert_eq!(lane.id, "to_c");
    assert_eq!(lane.nodes, ["p", "f", "F", "c"]);
    assert_eq!(lane.player_home_index, 0);
}

#[test]
fn links_not_on_a_lane_become_edges() {
    let map = read(&demo()).map;
    let pairs: Vec<String> =
        map.edges.iter().map(|edge| format!("{}-{}", edge.node_a_id, edge.node_b_id)).collect();
    assert_eq!(pairs, ["p-s2", "s2-w1", "w1-F"]);
}

#[test]
fn nodes_on_no_lane_are_kept_off_lane() {
    assert_eq!(off_lane(&read(&demo()).map), ["s2", "w1"]);
}

#[test]
fn a_nodes_position_is_its_grid_cells_centre_and_its_tile_that_cell() {
    let map = read(&demo()).map;
    assert_eq!(node(&map, "p").position, [96.0, 160.0]);
    assert_eq!(node(&map, "p").tile, [1, 2]);
}

#[test]
fn types_fields_and_names_convert() {
    let mut export = demo();
    node_entry(&mut export, "f")["fields"]["resource_type"] = json!("STONE");
    let map = read(&export).map;
    let farm = node(&map, "f");
    assert_eq!(farm.node_type, NodeType::Resource);
    assert_eq!(farm.resource_type, crate::content::ResourceType::Stone);
    assert_eq!(farm.yield_food_per_tick, 6);
    assert_eq!(node(&map, "w1").node_type, NodeType::Waypoint);
}

#[test]
fn hidden_status_garrisons_and_factions_carry_over() {
    let map = read(&demo()).map;
    assert_eq!(node(&map, "s2").hidden_from_faction_ids, ["player"]);
    let fort = node(&map, "F");
    assert_eq!(fort.garrison, 2);
    assert_eq!(fort.garrison_units[0].faction_id, "kingdom"); // fell back to the owner
    assert_eq!(fort.garrison_units[0].patrol_route, ["s2"]);
    assert_eq!(map.factions.len(), 2);
    assert_eq!(map.faction_relations[0].stance, Stance::Hostile);
}

#[test]
fn a_missing_player_home_is_an_error() {
    let mut export = demo();
    node_entry(&mut export, "p")["owning_faction_id"] = Value::Null;
    assert!(any(&read(&export).errors, "player home"));
}

#[test]
fn an_unreachable_target_is_an_error_but_the_node_is_kept() {
    let mut export = demo();
    export["links"].as_array_mut().unwrap().retain(|link| link["b"] != "c");
    let read = read(&export);
    assert!(any(&read.errors, "'c'"));
    assert!(off_lane(&read.map).contains(&"c"));
}

#[test]
fn unknown_fields_and_prototype_sections_are_warnings() {
    let mut export = demo();
    node_entry(&mut export, "f")["fields"]["mystery"] = json!(7);
    let read = read(&export);
    assert!(read.errors.is_empty(), "{:?}", read.errors);
    assert!(any(&read.warnings, "mystery"));
    assert!(any(&read.warnings, "structures"));
}

#[test]
fn a_wrong_format_or_version_is_refused() {
    let mut export = demo();
    export["format_version"] = json!(2);
    assert!(any(&read(&export).errors, "format"));
}

#[test]
fn a_garrison_unit_with_no_faction_anywhere_is_an_error() {
    let mut export = demo();
    node_entry(&mut export, "F")["owning_faction_id"] = Value::Null;
    assert!(any(&read(&export).errors, "'F'"));
}

#[test]
fn the_layout_carries_the_grid() {
    let read = read(&layout_map());
    assert!(read.errors.is_empty(), "{:?}", read.errors);
    let layout = read.map.layout.unwrap();
    assert_eq!((layout.cols, layout.rows, layout.cell_size), (12, 5, 64));
    assert_eq!(layout.default_terrain_id, "FIELDS");
}

#[test]
fn tiles_carry_terrain_feature_bridge_and_upgrades() {
    let layout = read(&layout_map()).map.layout.unwrap();
    let at = |cell: [i64; 2]| layout.tiles.iter().find(|tile| tile.cell == cell).unwrap();
    assert_eq!(layout.tiles.len(), 6);
    assert_eq!(at([2, 3]).terrain_id, "FOREST");
    assert_eq!(at([9, 4]).feature_id, "ORE_VEIN");
    let farm = at([4, 2]);
    assert_eq!(farm.terrain_id, ""); // the designer's "base terrain" flag
    assert_eq!(farm.upgrade_slots, 1);
    assert_eq!(farm.upgrade_ids, ["GUARD_BARRACKS"]);
    assert_eq!(farm.stability, -1);
    assert_eq!(at([7, 3]).bridge.as_ref().unwrap().hp, 40);
}

#[test]
fn roads_and_routes_carry_over() {
    let layout = read(&layout_map()).map.layout.unwrap();
    assert_eq!(layout.roads.len(), 7);
    assert_eq!(layout.roads[0].a, [1, 2]);
    assert_eq!(layout.routes.len(), 6);
    let first = &layout.routes[0];
    assert_eq!((first.node_a_id.as_str(), first.node_b_id.as_str()), ("p", "f"));
    assert_eq!((first.cells[0], *first.cells.last().unwrap()), ([1, 2], [4, 2]));
    assert_eq!(first.cost, 1.5);
}

#[test]
fn critical_assets_and_loss_groups_carry_over() {
    let map = read(&layout_map()).map;
    assert!(node(&map, "p").is_critical_asset && node(&map, "c").is_critical_asset);
    assert!(!node(&map, "f").is_critical_asset);
    assert_eq!(map.loss_groups.len(), 2);
    let home = &map.loss_groups[0];
    assert_eq!((home.faction_id.as_str(), home.display_name.as_str()), ("player", "Home"));
    assert_eq!(home.rule, LossRule::Any);
    assert_eq!(home.node_ids, ["p"]);
}

#[test]
fn only_structure_sections_are_still_not_read() {
    let warnings = read(&layout_map()).warnings;
    assert!(any(&warnings, "structures"));
    for section in ["tiles", "roads", "terrain_library", "loss_criteria", "link routes"] {
        assert!(!any(&warnings, section), "{section}");
    }
}

#[test]
fn a_terrain_missing_from_the_shared_library_is_an_error() {
    let mut export = layout_map();
    export["tiles"][0]["terrain"] = json!("NOT_A_TERRAIN");
    let errors = read(&export).errors;
    assert!(any(
        &errors,
        "tile (2, 3): terrain 'NOT_A_TERRAIN' is not in the shared terrain library"
    ));
}

#[test]
fn an_unknown_loss_rule_is_an_error() {
    let mut export = layout_map();
    export["loss_criteria"][0]["rule"] = json!("SOME");
    assert!(any(&read(&export).errors, "unknown rule 'SOME'"));
}

#[test]
fn an_export_without_a_grid_has_no_layout() {
    let mut export = layout_map();
    for section in ["grid", "tiles", "roads"] {
        export.as_object_mut().unwrap().remove(section);
    }
    assert!(read(&export).map.layout.is_none());
}

#[test]
fn a_tiles_elevation_and_the_maps_ceiling_carry_over() {
    let mut export = layout_map();
    export["ceiling"] = json!(48);
    export["tiles"][0]["elevation_override"] = json!(12);
    let layout = read(&export).map.layout.unwrap();
    assert_eq!((layout.ceiling, layout.tiles[0].elevation), (48, 12));
    let plain = read(&layout_map()).map.layout.unwrap();
    assert_eq!((plain.ceiling, plain.tiles[0].elevation), (64, -1));
}

#[test]
fn the_seed_and_channels_carry_over() {
    let mut export = layout_map();
    export["seed"] = json!(1234);
    export["channels"] = json!([{"tiles": [{"row": 0, "col": 0}, {"row": 0, "col": 1}],
        "width": 4, "depth": 3, "liquid": "WATER", "liquid_depth": 2}]);
    let read = read(&export);
    let layout = read.map.layout.unwrap();
    assert_eq!(layout.seed, 1234);
    assert_eq!(layout.channels[0].tiles, [[0, 0], [1, 0]]);
    assert_eq!((layout.channels[0].depth, layout.channels[0].liquid_id.as_str()), (3, "WATER"));
    assert!(read.errors.is_empty(), "{:?}", read.errors);
    let plain = self::read(&layout_map()).map.layout.unwrap();
    assert_eq!(plain.seed, 0);
    assert!(plain.channels.is_empty());
}

fn designer_plan() -> Value {
    json!({
        "solid_cells": {"3,4,0": "ROCK"},
        "faces": [
            {"x": 1, "y": 1, "level": 0, "side": "south", "material": "TIMBER", "thickness": 2},
            {"x": 1, "y": 1, "level": 1, "side": "floor", "material": "TIMBER", "thickness": 8}
        ],
        "dug": [[2, 2, -1]],
        "loads": {"1,1,1": 40}
    })
}

#[test]
fn a_plan_has_its_cells_faces_dug_cells_and_loads() {
    let plan = plan::build(&designer_plan(), "n", &mut Vec::new());
    assert_eq!(plan.solid_cells, [([3, 4, 0], "ROCK".to_string())]);
    let south = &plan.faces[0]; // a south face is stored as its neighbour's north face
    assert_eq!((south.cell, south.side, south.thickness), ([1, 2, 0], Side::North, 2));
    assert_eq!((plan.faces[1].side, plan.faces[1].thickness), (Side::Floor, 8));
    assert_eq!(plan.dug, [[2, 2, -1]]);
    assert_eq!(plan.loads, [([1, 1, 1], 40)]);
}

#[test]
fn flush_walls_and_fills_come_from_the_designer_plan() {
    let mut drawn = designer_plan();
    drawn["faces"].as_array_mut().unwrap().push(json!({"x": 4, "y": 4, "level": 0,
        "side": "north", "material": "TIMBER", "thickness": 1, "into_neighbour": true}));
    drawn["fills"] = json!({"2,2,-1": "WATER"});
    let plan = plan::build(&drawn, "n", &mut Vec::new());
    assert!(plan.faces[0].into_neighbour); // drawn as a south wall
    assert!(plan.faces[2].into_neighbour); // flipped explicitly
    assert!(!plan.faces[1].into_neighbour);
    assert_eq!(plan.fills, [([2, 2, -1], "WATER".to_string())]);
}

#[test]
fn an_older_plans_north_wall_grows_into_its_own_cell() {
    let drawn =
        json!({"faces": [{"x": 0, "y": 1, "level": 0, "side": "north", "material": "TIMBER"}]});
    assert!(!plan::build(&drawn, "n", &mut Vec::new()).faces[0].into_neighbour);
}

#[test]
fn a_face_on_a_side_that_isnt_one_is_an_error() {
    let drawn =
        json!({"faces": [{"x": 0, "y": 1, "level": 0, "side": "up", "material": "TIMBER"}]});
    let mut errors = Vec::new();
    assert!(plan::build(&drawn, "n", &mut errors).faces.is_empty());
    assert!(any(&errors, "unknown side 'up'"));
}

#[test]
fn a_nodes_plan_comes_with_it_and_a_node_without_one_has_none() {
    let mut export = layout_map();
    export["nodes"][0]["plan"] = designer_plan();
    let first = export["nodes"][0]["id"].as_str().unwrap().to_string();
    let map = read(&export).map;
    assert_eq!(node(&map, &first).plan.as_ref().unwrap().faces.len(), 2);
    assert!(read(&layout_map()).map.nodes[0].plan.is_none());
}

#[test]
fn a_nodes_tile_and_footprint_come_with_it() {
    let mut export = layout_map();
    export["nodes"][3]["footprint"] = json!([{"row": 2, "col": 9}, {"row": 3, "col": 9},
        {"row": 3, "col": 10}]); // c, at row 2, col 9
    let map = read(&export).map;
    let c = node(&map, "c");
    assert_eq!(c.tile, [9, 2]);
    assert_eq!(c.footprint, [[9, 2], [9, 3], [10, 3]]);
}

#[test]
fn a_nodes_subnodes_come_with_it() {
    let mut export = layout_map();
    export["nodes"][3]["subnodes"] = json!([{"id": "well_1", "type": "WELL", "at": [5, 6],
        "capture": [[5, 6], [5, 7]], "zone": [[5, 6], [5, 7], [4, 6]]}]);
    let map = read(&export).map;
    let well = &node(&map, "c").subnodes[0];
    assert_eq!((well.id.as_str(), well.subnode_type.as_str(), well.at), ("well_1", "WELL", [5, 6]));
    assert_eq!(well.capture_cells, [[5, 6], [5, 7]]);
    assert_eq!(well.zone_cells, [[5, 6], [5, 7], [4, 6]]);
    assert!(read(&layout_map()).map.nodes.iter().all(|node| node.subnodes.is_empty()));
}

#[test]
fn each_saved_designer_map_read_equals_the_checked_in_map() {
    let content = Content::load(&repo().join("content")).unwrap_or_else(|error| panic!("{error}"));
    let maps_src = repo().join("content/maps_src");
    for name in ["demo_map", "skirmish_p_c", "skirmish_two_lanes"] {
        let path = maps_src.join(format!("{name}{}", terrain::MAP_SUFFIX));
        let export: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let read = map_from_export(&export, &content.terrain);
        assert!(read.errors.is_empty(), "{name}: {:?}", read.errors);
        assert!(read.map == content.maps[name], "{name} differs from content/maps/{name}.json");
    }
}

/// The designer's terrain library with one of each entry (2b's test library).
fn designer_library() -> Value {
    json!({
        "terrains": [{
            "id": "DESERT", "label": "Desert", "glyph": ".", "color": "#d8c27a",
            "can_be_base": true, "needs_bridge": false, "move_cost": 1.5,
            "blocks_unit_classes": "cavalry, siege", "default_stability": 2,
            "default_max_height": 1, "default_max_width": 2, "default_max_depth": 0,
            "default_max_length": 2, "default_elevation": 3,
            "relief": {"amplitude": 2, "scale": 8}, "bearing": 3, "foundation_max": 6,
            "dig_depth": 24, "strata": [{"material": "SAND", "min": 2, "max": 6}],
            "liquids": [{"material": "WATER", "min": 16, "max": 24, "chance": 0.2,
                "surface_chance": 0}]
        }],
        "materials": [{
            "id": "TIMBER", "label": "Timber", "color": "#7a5a33", "weight": 2, "span": 4,
            "strength": 6, "heat": 1, "traits": {"dig_difficulty": 1},
            "heat_transitions": [{"above": 3, "gains": "burning"}]
        }],
        "features": [{"id": "ORE", "label": "Ore vein", "glyph": "o", "stub_effect": "metal"}],
        "road_move_multiplier": 0.5
    })
}

#[test]
fn a_terrain_takes_gdscripts_names() {
    let desert = &to_schema(&designer_library()).unwrap().terrains[0];
    assert_eq!((desert.display_name.as_str(), desert.move_cost), ("Desert", 1.5));
    assert_eq!(desert.blocks_unit_classes, ["cavalry", "siege"]);
    assert_eq!((desert.relief_amplitude, desert.relief_scale, desert.default_elevation), (2, 8, 3));
    let band = &desert.strata[0];
    assert_eq!((band.material_id.as_str(), band.min_cells, band.max_cells), ("SAND", 2, 6));
    let water = &desert.liquids[0];
    assert_eq!((water.material_id.as_str(), water.chance), ("WATER", 0.2));
    assert_eq!(desert.default_max_length, 2);
}

#[test]
fn a_material_its_heat_transitions_and_a_feature_carry_over() {
    let library = to_schema(&designer_library()).unwrap();
    let timber = &library.materials[0];
    assert_eq!((timber.display_name.as_str(), timber.strength), ("Timber", 6));
    let rule = &timber.heat_transitions[0];
    assert_eq!((rule.threshold, rule.rising, rule.gains_trait.as_str()), (3, true, "burning"));
    let ore = &library.features[0];
    assert_eq!((ore.display_name.as_str(), ore.effect_notes.as_str()), ("Ore vein", "metal"));
}

#[test]
fn omitted_ground_fields_take_the_imports_defaults() {
    let library = to_schema(&json!({"terrains": [{"id": "OLD", "bearing": 4},
        {"id": "T", "strata": [{"material": "SOIL", "min": 3}]}]}))
    .unwrap();
    let old = &library.terrains[0];
    assert_eq!((old.display_name.as_str(), old.foundation_max), ("OLD", 4));
    assert!(old.liquids.is_empty() && old.strata.is_empty());
    assert_eq!(library.terrains[1].strata[0].max_cells, 3);
}

#[test]
fn a_water_table_saved_before_decision_62_becomes_a_water_body() {
    let old = json!({"terrains": [{"id": "FIELDS", "water_table": {"min": 12, "max": 24}}]});
    let body = &to_schema(&old).unwrap().terrains[0].liquids[0];
    assert_eq!((body.material_id.as_str(), body.min_cells, body.max_cells), ("WATER", 12, 24));
}

#[test]
fn liquids_and_temperatures_saved_before_decisions_64_and_66_convert() {
    let old = json!({"liquids": [{"id": "LAVA", "temperature": 1200}], "materials": [
        {"id": "ROCK", "loose": true, "dig_difficulty": 3,
         "heat_transitions": [{"above": 1100, "becomes": "LAVA"}]}]});
    let library = to_schema(&old).unwrap();
    let find = |id: &str| library.materials.iter().find(|material| material.id == id).unwrap();
    assert_eq!(find("LAVA").heat, 8);
    assert_eq!(find("LAVA").traits, [("flows".to_string(), 1)].into());
    let rock = find("ROCK");
    assert_eq!(rock.traits, [("dig_difficulty".to_string(), 3), ("loose".to_string(), 1)].into());
    assert_eq!(rock.heat_transitions[0].threshold, 8);
}

#[test]
fn an_entry_without_an_id_or_with_a_colour_that_isnt_one_is_refused() {
    let nameless = to_schema(&json!({"terrains": [{"label": "Nameless"}]})).unwrap_err();
    assert!(nameless.contains("terrain entry has no id"), "{nameless}");
    let grey = to_schema(&json!({"materials": [{"id": "ROCK", "color": "grey"}]})).unwrap_err();
    assert!(grey.contains("materials ROCK: color"), "{grey}");
}

#[test]
fn the_designers_library_survives_the_schema_and_back() {
    let library = to_schema(&designer_library()).unwrap();
    assert_eq!(to_schema(&from_schema(&library)).unwrap(), library);
}

#[test]
fn the_repos_library_is_the_designers_written_back() {
    let checked_in = library();
    assert_eq!(to_schema(&from_schema(&checked_in)).unwrap(), checked_in);
    for terrain in checked_in.terrains.iter().filter(|terrain| terrain.can_be_base) {
        assert!(!terrain.strata.is_empty() && terrain.dig_depth > 0, "{}", terrain.id);
    }
}

/// A saved map using FIELDS, FOREST and ORE, in a fresh directory.
fn maps_using_forest(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("breach-designer-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let demo = json!({"default_terrain": "FIELDS", "tiles": [{"terrain": "FOREST", "feature": "ORE"},
        {"terrain": "FIELDS", "terrain_is_base": true}]});
    fs::write(dir.join("demo_map.designer.json"), demo.to_string()).unwrap();
    dir
}

#[test]
fn ids_used_by_saved_maps_are_found() {
    let (terrains, features) = used_ids(&maps_using_forest("used"));
    let demo = vec!["demo_map".to_string()];
    assert_eq!(
        terrains,
        [("FIELDS".to_string(), demo.clone()), ("FOREST".to_string(), demo.clone())]
    );
    assert_eq!(features, [("ORE".to_string(), demo)]);
}

#[test]
fn dropping_a_terrain_a_saved_map_uses_is_refused() {
    let library =
        to_schema(&json!({"terrains": [{"id": "FIELDS"}], "features": [{"id": "ORE"}]})).unwrap();
    let problems = in_use_problems(&library, &maps_using_forest("in-use"));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("terrain 'FOREST' is still used by demo_map"), "{problems:?}");
}
