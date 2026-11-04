//! Load paths (Decision 61, spec 24): every element carries its weight and what rests on it
//! down to the ground; it fails beyond its capacity or its span, and a ground column fails
//! beyond its bearing. `settle` follows the collapse through. The shared cases
//! (`tests/fixtures/structure/load_cases.json`) run here and against the designer's
//! `tools/designer/load_paths.js`, so the two stay one rule.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{settle, solve, Bearing};
use crate::content::{Material, Side, StructureFace, StructurePlan, TerrainLibrary};
use crate::designer::plan_from_export;

const FIELDS_BEARING: i64 = 4;
const CASES: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/structure/load_cases.json");

fn material(id: &str, weight: i64, strength: i64, span: i64) -> Material {
    Material { id: id.into(), weight, strength, span, ..Material::default() }
}

fn library() -> TerrainLibrary {
    TerrainLibrary {
        materials: vec![material("TIMBER", 1, 6, 3), material("ROCK", 2, 20, 4)],
        ..TerrainLibrary::default()
    }
}

/// A timber face drawn from the cell [x, y, level] on its side (a south or east face is its
/// neighbour's north or west).
fn face(x: i64, y: i64, level: i64, side: &str, thickness: i64) -> StructureFace {
    let (cell, side) = match side {
        "south" => ([x, y + 1, level], Side::North),
        "east" => ([x + 1, y, level], Side::West),
        "west" => ([x, y, level], Side::West),
        "floor" => ([x, y, level], Side::Floor),
        _ => ([x, y, level], Side::North),
    };
    StructureFace { cell, side, material_id: "TIMBER".into(), thickness, into_neighbour: false }
}

/// A 3-long timber face wall along the north of row 1, two levels high.
fn wall() -> StructurePlan {
    let mut plan = StructurePlan::default();
    for level in 0..2 {
        for x in 0..3 {
            plan.faces.push(face(x, 1, level, "north", 1));
        }
    }
    plan
}

/// A one-cell timber room, two levels high, roofed with a timber floor 8 eighths thick.
fn room() -> StructurePlan {
    let mut plan = StructurePlan::default();
    for level in 0..2 {
        for side in ["north", "south", "west", "east"] {
            plan.faces.push(face(0, 0, level, side, 1));
        }
    }
    plan.faces.push(face(0, 0, 2, "floor", 8));
    plan
}

fn failed(plan: &StructurePlan, bearing: i64) -> Vec<String> {
    solve(plan, &library(), &Bearing::every(bearing)).failed
}

#[test]
fn a_timber_wall_with_no_floor_stands() {
    assert!(failed(&wall(), FIELDS_BEARING).is_empty());
}

#[test]
fn a_wall_bridges_a_dug_gap_beneath_it() {
    let mut plan = wall();
    plan.dug = vec![[1, 1, -1], [1, 0, -1]];

    let solved = solve(&plan, &library(), &Bearing::every(FIELDS_BEARING));

    assert!(solved.failed.is_empty());
    assert_eq!(solved.loads["face:0,1,0,north"], 3); // its own 2, and half the middle's
}

#[test]
fn a_roof_stands_but_a_ballista_on_it_brings_the_walls_down() {
    assert!(failed(&room(), FIELDS_BEARING).is_empty());

    let mut plan = room();
    plan.loads = vec![([0, 0, 2], 40)];
    let failed = failed(&plan, FIELDS_BEARING);

    assert!(failed.contains(&"face:0,0,1,north".to_string()));
    assert!(!failed.contains(&"face:0,0,2,floor".to_string())); // the roof itself holds
}

#[test]
fn a_floor_beyond_its_span_falls() {
    let plan = StructurePlan { faces: vec![face(5, 5, 1, "floor", 1)], ..StructurePlan::default() };

    assert_eq!(failed(&plan, FIELDS_BEARING), ["face:5,5,1,floor"]);
}

#[test]
fn the_grounds_bearing_limits_a_stone_tower() {
    let mut plan = StructurePlan::default();
    for level in 0..4 {
        plan.solid_cells.push(([0, 0, level], "ROCK".into()));
    }

    assert!(failed(&plan, 8).is_empty()); // 4 cells x 16 units = 64 = bearing 8 x 8

    plan.solid_cells.push(([0, 0, 4], "ROCK".into()));
    assert!(failed(&plan, 8).contains(&"cell:0,0,0".to_string()));
}

#[test]
fn failure_cascades_when_settled() {
    let plan = StructurePlan {
        faces: vec![face(5, 5, 1, "floor", 1)],
        solid_cells: vec![([5, 5, 1], "TIMBER".into())], // rests on that floor
        ..StructurePlan::default()
    };

    assert_eq!(failed(&plan, FIELDS_BEARING), ["face:5,5,1,floor"]);
    let fallen = settle(&plan, &library(), &Bearing::every(FIELDS_BEARING));
    assert_eq!(fallen, ["face:5,5,1,floor", "cell:5,5,1"]);
    assert_eq!(plan.faces.len(), 1); // settle leaves the plan itself unchanged
}

#[test]
fn a_cell_drawn_twice_is_one_element_where_it_was_first_drawn() {
    let plan = StructurePlan {
        solid_cells: vec![
            ([0, 0, 0], "TIMBER".into()),
            ([1, 0, 0], "ROCK".into()),
            ([0, 0, 0], "ROCK".into()),
        ],
        ..StructurePlan::default()
    };

    let solved = solve(&plan, &library(), &Bearing::every(FIELDS_BEARING));

    assert_eq!(solved.loads["cell:0,0,0"], 16); // the last drawing's material, ROCK
    assert_eq!(solved.loads.len(), 4); // two cells and the ground under each
}

/// A case's materials, as the fixture gives them (`flows` a trait).
fn fixture_library(entries: &[Value]) -> TerrainLibrary {
    let materials = entries
        .iter()
        .map(|entry| {
            let int = |key: &str| entry[key].as_i64().unwrap_or(0);
            let mut made = material(
                entry["id"].as_str().unwrap_or(""),
                int("weight"),
                int("strength"),
                int("span"),
            );
            if entry.get("flows").is_some() {
                made.traits = BTreeMap::from([("flows".to_string(), int("flows"))]);
            }
            made
        })
        .collect();
    TerrainLibrary { materials, ..TerrainLibrary::default() }
}

/// A case's bearing: one number for every column, or {default, columns: {"x,y": n}}.
fn fixture_bearing(given: &Value) -> Bearing {
    let Some(given) = given.as_object() else {
        return Bearing::every(given.as_i64().unwrap_or(0));
    };
    let mut columns = BTreeMap::new();
    for (at, bearing) in given.get("columns").and_then(Value::as_object).into_iter().flatten() {
        let parts: Vec<i64> = at.split(',').map(|part| part.parse().unwrap_or(0)).collect();
        columns.insert([parts[0], parts[1]], bearing.as_i64().unwrap_or(0));
    }
    Bearing { default: given.get("default").and_then(Value::as_i64).unwrap_or(0), columns }
}

fn strings(value: &Value) -> Vec<String> {
    let list = value.as_array().map_or(&[][..], Vec::as_slice);
    list.iter().map(|key| key.as_str().unwrap_or("").to_string()).collect()
}

#[test]
fn every_shared_case_holds() {
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(CASES).expect("the shared cases")).unwrap();
    let library = fixture_library(fixture["materials"].as_array().unwrap());
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap_or("");
        let (plan, errors) = plan_from_export(&case["plan"]);
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let bearing = fixture_bearing(&case["bearing"]);
        let solved = solve(&plan, &library, &bearing);
        assert_eq!(solved.failed, strings(&case["failed"]), "{name}");
        for (key, load) in case.get("loads").and_then(Value::as_object).into_iter().flatten() {
            assert_eq!(solved.loads.get(key).copied(), load.as_i64(), "{name}: {key}");
        }
        if let Some(settled) = case.get("settled") {
            assert_eq!(settle(&plan, &library, &bearing), strings(settled), "{name}");
        }
    }
}
