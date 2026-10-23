//! A map's nodes from the export's "nodes" (spec 16): each at its grid position's cell
//! centre, with its footprint, its designer "fields" set by name, its garrison units, plan
//! and subnodes.

use serde_json::{Map as Fields, Value};

use super::{field_text, flag_of, int, list, named, plan, real, strings, text};
use crate::content::{GarrisonUnit, Node, ResourceType, Subnode};

const DEFAULT_CELL_SIZE: f64 = 64.0;
/// Node fields the builder sets itself: a designer "fields" key never overrides them.
const RESERVED: [&str; 7] = [
    "id",
    "node_type",
    "position",
    "garrison",
    "garrison_units",
    "owning_faction_id",
    "hidden_from_faction_ids",
];

/// Every node in the export, in its order; a later node with the same id replaces the
/// earlier one in its place.
pub fn build(export: &Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) -> Vec<Node> {
    let cell = export.get("cell_size").map_or(DEFAULT_CELL_SIZE, real);
    let mut nodes: Vec<Node> = Vec::new();
    for entry in list(export, "nodes") {
        let node = node(entry, cell, errors, warnings);
        match nodes.iter_mut().find(|earlier| earlier.id == node.id) {
            Some(earlier) => *earlier = node,
            None => nodes.push(node),
        }
    }
    nodes
}

/// The designer's {row, col} as a (col, row) cell.
pub fn cell_of(grid_position: Option<&Value>) -> [i64; 2] {
    let at = |key| grid_position.and_then(|place| place.get(key)).map_or(0, int);
    [at("col"), at("row")]
}

fn node(entry: &Value, cell: f64, errors: &mut Vec<String>, warnings: &mut Vec<String>) -> Node {
    let mut node = Node { id: field_text(entry, "id"), ..Node::default() };
    let type_name = field_text(entry, "node_type");
    match named(&type_name) {
        Some(node_type) => node.node_type = node_type,
        None => errors.push(format!("node '{}': unknown node_type '{type_name}'", node.id)),
    }
    node.owning_faction_id = field_text(entry, "owning_faction_id");
    node.hidden_from_faction_ids = strings(entry.get("hidden_from_faction_ids"));
    node.is_critical_asset = flag_of(entry, "is_critical_asset");
    let place = entry.get("grid_position");
    let at = |key| place.and_then(|place| place.get(key)).map_or(0.0, real);
    node.position = [(at("col") + 0.5) * cell, (at("row") + 0.5) * cell];
    node.tile = cell_of(place);
    node.footprint = list(entry, "footprint").iter().map(|place| cell_of(Some(place))).collect();
    if let Some(fields) = entry.get("fields").and_then(Value::as_object) {
        node = with_fields(node, fields, errors, warnings);
    }
    garrison(&mut node, list(entry, "garrison_units"), errors);
    if let Some(drawn) = entry.get("plan").filter(|drawn| drawn.is_object()) {
        node.plan = Some(plan::build(drawn, &node.id, errors));
    }
    node.subnodes = list(entry, "subnodes").iter().map(subnode).collect();
    node
}

/// A designer subnode: cells are [x, y] in the node's local cells (spec 26).
fn subnode(entry: &Value) -> Subnode {
    Subnode {
        id: field_text(entry, "id"),
        subnode_type: field_text(entry, "type"),
        at: pair(entry.get("at")),
        capture_cells: list(entry, "capture").iter().map(|cell| pair(Some(cell))).collect(),
        zone_cells: list(entry, "zone").iter().map(|cell| pair(Some(cell))).collect(),
    }
}

fn pair(value: Option<&Value>) -> [i64; 2] {
    match value.and_then(Value::as_array) {
        Some(pair) if pair.len() >= 2 => [int(&pair[0]), int(&pair[1])],
        _ => [0, 0],
    }
}

/// Sets each of the designer's `fields` on the node by name, as its field's type; a field
/// the node doesn't have, or one the builder sets, is skipped with a warning.
fn with_fields(
    node: Node,
    fields: &Fields<String, Value>,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Node {
    let id = node.id.clone();
    let Ok(Value::Object(mut built)) = serde_json::to_value(&node) else {
        return node;
    };
    for (name, value) in fields {
        if name == "resource_type" {
            match named::<ResourceType>(&text(value)) {
                Some(_) => {
                    built.insert(name.clone(), Value::String(text(value)));
                }
                None => {
                    errors.push(format!("node '{id}': unknown resource_type '{}'", text(value)))
                }
            }
            continue;
        }
        let Some(current) = built.get(name).filter(|_| !RESERVED.contains(&name.as_str())) else {
            warnings
                .push(format!("node '{id}': field '{name}' is not a NodeDef property; skipped"));
            continue;
        };
        let set = match current {
            Value::Number(number) if number.is_i64() => Value::from(int(value)),
            Value::Number(_) => Value::from(real(value)),
            Value::Bool(_) => Value::Bool(super::flag(value)),
            Value::String(_) => Value::String(text(value)),
            _ => {
                warnings
                    .push(format!("node '{id}': field '{name}' has an unsupported type; skipped"));
                continue;
            }
        };
        built.insert(name.clone(), set);
    }
    serde_json::from_value(Value::Object(built)).unwrap_or(node)
}

/// The node's stationed units; a unit with no faction serves the node's owner.
fn garrison(node: &mut Node, units: &[Value], errors: &mut Vec<String>) {
    node.garrison_units = units
        .iter()
        .map(|entry| {
            let mut faction_id = field_text(entry, "faction_id");
            if faction_id.is_empty() {
                faction_id = node.owning_faction_id.clone();
            }
            if faction_id.is_empty() {
                errors.push(format!(
                    "node '{}': a garrison unit has no faction and the node has no owner",
                    node.id
                ));
            }
            GarrisonUnit {
                faction_id,
                can_sortie: flag_of(entry, "can_sortie"),
                patrol_route: strings(entry.get("patrol_route")),
                delivery_target_id: field_text(entry, "delivery_target_id"),
            }
        })
        .collect();
    node.garrison = node.garrison_units.len() as i64;
}
