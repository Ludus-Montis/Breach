//! A designer structure plan (spec 24) as the schema's: solid cells, fills and loads keyed
//! "x,y,level", faces with any side name (a south or east face is stored as its neighbour's
//! north or west face, growing back into this cell), and dug cells as [x, y, level]. A face's
//! `into_neighbour`, when given, overrides its side's default.

use serde_json::Value;

use super::{flag, int, list, text};
use crate::content::{Side, StructureFace, StructurePlan};

pub fn build(drawn: &Value, node_id: &str, errors: &mut Vec<String>) -> StructurePlan {
    StructurePlan {
        solid_cells: keyed(drawn.get("solid_cells"), text),
        faces: list(drawn, "faces")
            .iter()
            .filter_map(|face| face_of(face, node_id, errors))
            .collect(),
        dug: list(drawn, "dug")
            .iter()
            .map(|dug| cell3(dug.as_array().map_or(&[], Vec::as_slice)))
            .collect(),
        fills: keyed(drawn.get("fills"), text),
        loads: keyed(drawn.get("loads"), int),
    }
}

/// An object keyed "x,y,level", in its order, each value read by `read`.
fn keyed<T>(value: Option<&Value>, read: fn(&Value) -> T) -> Vec<([i64; 3], T)> {
    let Some(entries) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    entries
        .iter()
        .map(|(at, value)| {
            let parts: Vec<Value> =
                at.split(',').map(|part| Value::String(part.to_string())).collect();
            (cell3(&parts), read(value))
        })
        .collect()
}

fn cell3(parts: &[Value]) -> [i64; 3] {
    let at = |index: usize| parts.get(index).map_or(0, int);
    [at(0), at(1), at(2)]
}

/// The face drawn from `face`'s cell on its side; none, with an error, for a side that
/// isn't north, south, east, west or floor.
fn face_of(face: &Value, node_id: &str, errors: &mut Vec<String>) -> Option<StructureFace> {
    let at = |key| face.get(key).map_or(0, int);
    let [x, y, level] = [at("x"), at("y"), at("level")];
    let side_name = face.get("side").map_or("north".to_string(), text);
    let (cell, side) = match side_name.as_str() {
        "north" => ([x, y, level], Side::North),
        "west" => ([x, y, level], Side::West),
        "floor" => ([x, y, level], Side::Floor),
        "south" => ([x, y + 1, level], Side::North),
        "east" => ([x + 1, y, level], Side::West),
        _ => {
            errors.push(format!("node '{node_id}': a face has an unknown side '{side_name}'"));
            return None;
        }
    };
    Some(StructureFace {
        cell,
        side,
        material_id: face.get("material").map_or(String::new(), text),
        thickness: face.get("thickness").map_or(1, int),
        into_neighbour: face
            .get("into_neighbour")
            .map_or(matches!(side_name.as_str(), "south" | "east"), flag),
    })
}
