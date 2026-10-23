//! A map's layout from the export (spec 19): the grid, default terrain, tiles, roads, each
//! link's route and the channels. Terrain and feature ids must be in the shared terrain
//! library (Decision 32); a missing one is an error naming the cell, since the designer should
//! have saved its terrain library first.

use serde_json::Value;

use super::nodes::cell_of;
use super::{field_text, flag_of, int, int_or, list, real, text};
use crate::content::{Bridge, Channel, MapLayout, RoadSegment, Route, TerrainLibrary, Tile};

const MISSING: &str =
    "is not in the shared terrain library - save the terrain library in the designer";

/// None when the export has no grid (nothing spatial was authored).
pub fn build(
    export: &Value,
    library: &TerrainLibrary,
    errors: &mut Vec<String>,
) -> Option<MapLayout> {
    let grid = export.get("grid").and_then(Value::as_object).filter(|grid| !grid.is_empty())?;
    let size = |key| grid.get(key).map_or(0, int);
    let default_terrain_id = export.get("default_terrain").map_or(String::new(), text);
    if !has_terrain(library, &default_terrain_id) {
        errors.push(format!("default terrain '{default_terrain_id}' {MISSING}"));
    }
    Some(MapLayout {
        cols: size("cols"),
        rows: size("rows"),
        cell_size: int_or(export, "cell_size", 64),
        default_terrain_id,
        ceiling: int_or(export, "ceiling", 64),
        seed: int_or(export, "seed", 0),
        channels: list(export, "channels").iter().map(channel).collect(),
        tiles: list(export, "tiles").iter().map(|entry| tile(entry, library, errors)).collect(),
        roads: list(export, "roads")
            .iter()
            .map(|entry| RoadSegment { a: cell_of(entry.get("a")), b: cell_of(entry.get("b")) })
            .collect(),
        routes: list(export, "links")
            .iter()
            .filter(|entry| !list(entry, "route").is_empty())
            .map(route)
            .collect(),
    })
}

fn has_terrain(library: &TerrainLibrary, id: &str) -> bool {
    library.terrains.iter().any(|terrain| terrain.id == id)
}

/// A designer tile; "(col, row)" names its cell in an error, as Godot printed a Vector2i.
fn tile(entry: &Value, library: &TerrainLibrary, errors: &mut Vec<String>) -> Tile {
    let mut tile = Tile { cell: cell_of(entry.get("grid_position")), ..Tile::default() };
    let at = format!("({}, {})", tile.cell[0], tile.cell[1]);
    if !flag_of(entry, "terrain_is_base") {
        tile.terrain_id = field_text(entry, "terrain");
        if !has_terrain(library, &tile.terrain_id) {
            errors.push(format!("tile {at}: terrain '{}' {MISSING}", tile.terrain_id));
        }
    }
    if let Some(feature) = entry.get("feature").filter(|feature| !feature.is_null()) {
        tile.feature_id = text(feature);
        if !library.features.iter().any(|known| known.id == tile.feature_id) {
            errors.push(format!("tile {at}: feature '{}' {MISSING}", tile.feature_id));
        }
    }
    if let Some(bridge) = entry.get("bridge").filter(|bridge| bridge.is_object()) {
        tile.bridge = Some(Bridge {
            owning_faction_id: field_text(bridge, "owning_faction_id"),
            hp: int_or(bridge, "hp", 40),
            demolishable_by_owner: flag_of(bridge, "demolishable_by_owner"),
            drawbridge_node_id: field_text(bridge, "drawbridge_node_id"),
            raised: flag_of(bridge, "raised"),
        });
    }
    let override_of = |key| entry.get(key).filter(|found| !found.is_null()).map(int);
    let overrides = [
        ("stability_override", &mut tile.stability),
        ("max_height_override", &mut tile.max_height),
        ("max_width_override", &mut tile.max_width),
        ("dig_depth_override", &mut tile.max_depth),
        ("elevation_override", &mut tile.elevation),
    ];
    for (key, field) in overrides {
        if let Some(value) = override_of(key) {
            *field = value;
        }
    }
    tile.upgrade_slots = int_or(entry, "upgrade_slots", 0);
    tile.upgrade_ids = list(entry, "upgrades").iter().map(text).collect();
    tile
}

fn channel(entry: &Value) -> Channel {
    Channel {
        tiles: list(entry, "tiles").iter().map(|place| cell_of(Some(place))).collect(),
        width: int_or(entry, "width", 4),
        depth: int_or(entry, "depth", 2),
        liquid_id: field_text(entry, "liquid"),
        liquid_depth: int_or(entry, "liquid_depth", 0),
    }
}

fn route(entry: &Value) -> Route {
    Route {
        node_a_id: field_text(entry, "a"),
        node_b_id: field_text(entry, "b"),
        cells: list(entry, "route").iter().map(|place| cell_of(Some(place))).collect(),
        cost: entry.get("route_cost").filter(|cost| !cost.is_null()).map_or(0.0, real),
    }
}
