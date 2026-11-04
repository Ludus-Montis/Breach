//! What holds up each element of a structure plan (Decision 61, spec 24), ported from
//! GDScript's StructureSupports as it was. Elements are keyed "cell:x,y,level" or
//! "face:x,y,level,side"; the ground under a column is "ground:x,y". Pure over the plan it is
//! given.
//! - A solid cell rests on the solid cell beneath, the floor under it, or the ground.
//! - A face wall rests on the same face a level down, a solid cell beneath either side, or
//!   the ground at level 0.
//! - A floor rests on the solid cell beneath, the face walls along its edges a level down, or
//!   the ground at level 0.
//!
//! Ground counts only where it isn't dug, or is dug and filled with a solid (Decision 67); a
//! liquid fill (a moat) holds nothing up.

use std::collections::{BTreeMap, BTreeSet};

use crate::content::{Cell3, Side, StructurePlan, TerrainLibrary};

const DOWN: Cell3 = [0, 0, -1];

/// What an element is made of and where: a solid cell, or a face on its side.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub at: Cell3,
    /// "cell", or the face's side: "north", "west" or "floor".
    pub kind: &'static str,
    pub material: String,
    /// 8 for a cell, else the face's thickness.
    pub eighths: i64,
}

/// Every element of a plan by key, in the order a keyed map of them would list them: the
/// solid cells first, then the faces, each where it was first drawn, with its last drawing.
#[derive(Clone, Debug, Default)]
pub struct Elements {
    pub order: Vec<String>,
    pub by_key: BTreeMap<String, Element>,
}

impl Elements {
    fn put(&mut self, key: String, element: Element) {
        if !self.by_key.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.by_key.insert(key, element);
    }

    pub fn get(&self, key: &str) -> &Element {
        &self.by_key[key]
    }
}

pub struct Supports {
    cells: BTreeSet<Cell3>,
    faces: BTreeSet<String>,
    dug: BTreeSet<Cell3>,
    elements: Elements,
}

impl Supports {
    pub fn new(plan: &StructurePlan, library: &TerrainLibrary) -> Supports {
        let mut elements = Elements::default();
        for (at, material) in &plan.solid_cells {
            let element = Element { at: *at, kind: "cell", material: material.clone(), eighths: 8 };
            elements.put(cell_key(*at), element);
        }
        let mut faces = BTreeSet::new();
        for face in &plan.faces {
            let kind = side_name(face.side);
            let key = face_key(face.cell, kind);
            faces.insert(key.clone());
            let material = face.material_id.clone();
            elements.put(key, Element { at: face.cell, kind, material, eighths: face.thickness });
        }
        let dug = plan
            .dug
            .iter()
            .filter(|at| {
                let fill = last(&plan.fills, at)
                    .and_then(|id| library.materials.iter().find(|material| &material.id == id));
                fill.is_none_or(|fill| fill.traits.get("flows").copied().unwrap_or(0) > 0)
            })
            .copied()
            .collect();
        let cells = plan.solid_cells.iter().map(|(at, _)| *at).collect();
        Supports { cells, faces, dug, elements }
    }

    pub fn elements(&self) -> &Elements {
        &self.elements
    }

    /// The keys an element rests on directly; none if nothing holds it but its neighbours.
    pub fn direct(&self, element: &Element) -> Vec<String> {
        let at = element.at;
        let below = add(at, DOWN);
        match element.kind {
            "cell" => first(&[self.cell(below), self.face(at, "floor"), self.ground(at, at)]),
            "floor" => {
                let walls: Vec<String> = [
                    self.face(below, "north"),
                    self.face(add(add(at, [0, 1, 0]), DOWN), "north"),
                    self.face(below, "west"),
                    self.face(add(add(at, [1, 0, 0]), DOWN), "west"),
                ]
                .into_iter()
                .filter(|key| !key.is_empty())
                .collect();
                if !self.cell(below).is_empty() {
                    return vec![self.cell(below)];
                }
                if walls.is_empty() {
                    first(&[self.ground(at, at)])
                } else {
                    walls
                }
            }
            side => {
                let beside = add(at, if side == "north" { [0, -1, 0] } else { [-1, 0, 0] });
                if !self.face(below, side).is_empty() {
                    return vec![self.face(below, side)];
                }
                let cells: Vec<String> = [self.cell(below), self.cell(add(beside, DOWN))]
                    .into_iter()
                    .filter(|key| !key.is_empty())
                    .collect();
                if cells.is_empty() {
                    first(&[self.ground(at, beside)])
                } else {
                    cells
                }
            }
        }
    }

    /// Elements of the same kind beside this one, which it can hang from within its span: the
    /// next face along a wall's run, the next floor, the next solid cell.
    pub fn neighbours(&self, element: &Element) -> Vec<String> {
        let steps: &[Cell3] = match element.kind {
            "north" => &[[1, 0, 0], [-1, 0, 0]],
            "west" => &[[0, 1, 0], [0, -1, 0]],
            _ => &[[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0]],
        };
        steps
            .iter()
            .map(|step| {
                let next = add(element.at, *step);
                if element.kind == "cell" {
                    self.cell(next)
                } else {
                    self.face(next, element.kind)
                }
            })
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// What a load resting in a cell bears on: its floor, the solid cell beneath, the ground;
    /// "" for nothing.
    pub fn load_target(&self, at: Cell3) -> String {
        let found = first(&[self.face(at, "floor"), self.cell(add(at, DOWN)), self.ground(at, at)]);
        found.into_iter().next().unwrap_or_default()
    }

    fn cell(&self, at: Cell3) -> String {
        if self.cells.contains(&at) {
            cell_key(at)
        } else {
            String::new()
        }
    }

    fn face(&self, at: Cell3, side: &str) -> String {
        let key = face_key(at, side);
        if self.faces.contains(&key) {
            key
        } else {
            String::new()
        }
    }

    /// The ground under one of two columns at level 0 (the first not dug out), or "".
    fn ground(&self, at: Cell3, other: Cell3) -> String {
        if at[2] != 0 {
            return String::new();
        }
        [at, other]
            .into_iter()
            .find(|column| !self.dug.contains(&add(*column, DOWN)))
            .map_or(String::new(), |column| format!("ground:{},{}", column[0], column[1]))
    }
}

/// The value last drawn for `at` in a keyed list, as a keyed map of it would hold.
pub fn last<'a, T>(keyed: &'a [(Cell3, T)], at: &Cell3) -> Option<&'a T> {
    keyed.iter().rev().find(|(cell, _)| cell == at).map(|(_, value)| value)
}

pub fn cell_key(at: Cell3) -> String {
    format!("cell:{},{},{}", at[0], at[1], at[2])
}

pub fn face_key(at: Cell3, side: &str) -> String {
    format!("face:{},{},{},{side}", at[0], at[1], at[2])
}

pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::North => "north",
        Side::West => "west",
        Side::Floor => "floor",
    }
}

fn first(keys: &[String]) -> Vec<String> {
    keys.iter().find(|key| !key.is_empty()).cloned().into_iter().collect()
}

fn add(at: Cell3, step: Cell3) -> Cell3 {
    [at[0] + step[0], at[1] + step[1], at[2] + step[2]]
}
