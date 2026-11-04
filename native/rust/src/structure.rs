//! Load paths through a structure plan (Decisions 61, 65, spec 24; spec 39 step 9a), in whole
//! load units, ported from GDScript's LoadPaths as it was.
//! - Each element weighs its material's weight x its eighths and carries its strength x its
//!   eighths; a ground column carries its bearing x 8. Bearing is one number for every
//!   column, or a default with some columns of their own, where a plan crosses tiles of
//!   different ground (Decision 68).
//! - Loads flow top-down: an element passes its weight plus whatever rests on it, split
//!   equally, to what holds it (`supports`). An element with nothing beneath hangs from
//!   directly held elements of its own kind within its material's span; with none in reach
//!   it fails.
//! - An element fails when its load exceeds its capacity; elements standing on a ground
//!   column fail together when the column is overloaded.
//!
//! `settle` removes failures and solves again until nothing more fails. Pure.

mod supports;

use std::collections::{BTreeMap, BTreeSet};

use crate::content::{Cell3, Material, StructurePlan, TerrainLibrary};
use supports::{Elements, Supports};

/// What a ground column bears: `default` under every column but those in `columns`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bearing {
    pub default: i64,
    pub columns: BTreeMap<[i64; 2], i64>,
}

impl Bearing {
    /// One bearing under every column.
    pub fn every(bearing: i64) -> Bearing {
        Bearing { default: bearing, columns: BTreeMap::new() }
    }
}

/// A solve: each element's and ground column's load by key, and the keys that failed, in
/// the order found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solved {
    pub loads: BTreeMap<String, i64>,
    pub failed: Vec<String>,
}

/// Every element's and ground column's load, and what fails.
pub fn solve(plan: &StructurePlan, library: &TerrainLibrary, bearing: &Bearing) -> Solved {
    let supports = Supports::new(plan, library);
    let elements = supports.elements();
    let mut held: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut failed = Vec::new();
    for key in &elements.order {
        held.insert(key.clone(), supports.direct(elements.get(key)));
    }
    let bridged = bridge(&supports, library, &mut held, &mut failed);
    let mut loads = BTreeMap::new();
    for key in &elements.order {
        let element = elements.get(key);
        let weight = material(library, &element.material).map_or(0, |found| found.weight);
        loads.insert(key.clone(), weight * element.eighths);
    }
    let mut placed: Vec<Cell3> = Vec::new();
    for (at, _) in &plan.loads {
        if placed.contains(at) {
            continue; // a keyed map holds a cell once, with its last load
        }
        placed.push(*at);
        let target = supports.load_target(*at);
        if !target.is_empty() {
            let load = *supports::last(&plan.loads, at).unwrap_or(&0);
            *loads.entry(target).or_insert(0) += load;
        }
    }
    for key in top_down(elements, &bridged) {
        let element = elements.get(&key);
        let strength = material(library, &element.material).map_or(0, |found| found.strength);
        if loads[&key] > strength * element.eighths && !failed.contains(&key) {
            failed.push(key.clone());
        }
        pass_down(&mut loads, &key, &held[&key]);
    }
    for key in &elements.order {
        for target in &held[key] {
            let bears = column_bearing(bearing, target) * 8;
            let over = loads.get(target).copied().unwrap_or(0) > bears;
            if target.starts_with("ground:") && over && !failed.contains(key) {
                failed.push(key.clone());
            }
        }
    }
    Solved { loads, failed }
}

/// Every element that fails as each failure brings down what it held; the plan is left as
/// it was (the solve runs on a copy).
pub fn settle(plan: &StructurePlan, library: &TerrainLibrary, bearing: &Bearing) -> Vec<String> {
    let mut working = plan.clone();
    let mut fallen = Vec::new();
    loop {
        let failed = solve(&working, library, bearing).failed;
        if failed.is_empty() {
            return fallen;
        }
        working.solid_cells.retain(|(at, _)| !failed.contains(&supports::cell_key(*at)));
        working.faces.retain(|face| {
            !failed.contains(&supports::face_key(face.cell, supports::side_name(face.side)))
        });
        fallen.extend(failed);
    }
}

/// Elements with nothing beneath hang from the nearest directly held elements of their kind
/// within their material's span; returns the keys that hang, and fails any that can't.
fn bridge(
    supports: &Supports,
    library: &TerrainLibrary,
    held: &mut BTreeMap<String, Vec<String>>,
    failed: &mut Vec<String>,
) -> BTreeSet<String> {
    let elements = supports.elements();
    let mut bridged = BTreeSet::new();
    for key in &elements.order {
        if !held[key].is_empty() {
            continue;
        }
        let reach = material(library, &elements.get(key).material).map_or(0, |found| found.span);
        let anchors = nearest_held(key, supports, held, reach);
        if anchors.is_empty() {
            failed.push(key.clone());
        }
        held.insert(key.clone(), anchors);
        bridged.insert(key.clone());
    }
    bridged
}

/// The directly held elements nearest to `start` along its own kind, within `reach` steps.
fn nearest_held(
    start: &str,
    supports: &Supports,
    held: &BTreeMap<String, Vec<String>>,
    reach: i64,
) -> Vec<String> {
    let elements = supports.elements();
    let mut seen = BTreeSet::from([start.to_string()]);
    let mut frontier = vec![start.to_string()];
    for _ in 0..reach {
        let mut next = Vec::new();
        let mut found = Vec::new();
        for key in &frontier {
            for neighbour in supports.neighbours(elements.get(key)) {
                if !seen.insert(neighbour.clone()) {
                    continue;
                }
                next.push(neighbour.clone());
                if !held[&neighbour].is_empty() && !hangs(held, &neighbour, elements) {
                    found.push(neighbour);
                }
            }
        }
        if !found.is_empty() {
            found.sort();
            return found;
        }
        frontier = next;
    }
    Vec::new()
}

/// True if an element's supports are only its own kind's neighbours (it hangs itself).
fn hangs(held: &BTreeMap<String, Vec<String>>, key: &str, elements: &Elements) -> bool {
    let level = elements.get(key).at[2];
    held[key]
        .iter()
        .any(|target| elements.by_key.get(target).is_some_and(|element| element.at[2] == level))
}

/// Highest level first; within a level, hanging elements before those they hang from.
fn top_down(elements: &Elements, bridged: &BTreeSet<String>) -> Vec<String> {
    let mut keys = elements.order.clone();
    keys.sort_by(|a, b| {
        let (la, lb) = (elements.get(a).at[2], elements.get(b).at[2]);
        lb.cmp(&la).then(bridged.contains(b).cmp(&bridged.contains(a))).then(a.cmp(b))
    });
    keys
}

/// Splits a load equally between targets; the remainder goes to the first, in key order.
fn pass_down(loads: &mut BTreeMap<String, i64>, key: &str, targets: &[String]) {
    if targets.is_empty() {
        return;
    }
    let mut ordered = targets.to_vec();
    ordered.sort();
    let count = ordered.len() as i64;
    let load = loads[key];
    for (index, target) in ordered.into_iter().enumerate() {
        let extra = if index == 0 { load % count } else { 0 };
        *loads.entry(target).or_insert(0) += load / count + extra;
    }
}

/// The bearing under a "ground:x,y" column.
pub fn column_bearing(bearing: &Bearing, ground_key: &str) -> i64 {
    let mut parts = ground_key.trim_start_matches("ground:").split(',');
    let mut next = || parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let column = [next(), next()];
    bearing.columns.get(&column).copied().unwrap_or(bearing.default)
}

/// The library's first material of that id, as `TerrainLibraryDef.material` finds it.
fn material<'a>(library: &'a TerrainLibrary, id: &str) -> Option<&'a Material> {
    library.materials.iter().find(|material| material.id == id)
}

#[cfg(test)]
mod tests;
