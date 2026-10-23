//! A map's lanes from the export's links (spec 16, the user's rule: links already define the
//! routes, so lanes come from them). Home is the first player-owned critical asset, else the
//! first player-owned ORIGIN. Each other faction's critical asset or origin gets one lane:
//! the fewest-link path from home (breadth first, ties to the earlier link). Every link that
//! isn't a consecutive pair in some lane becomes an edge. Nodes on no lane are off-lane.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde_json::Value;

use super::{field_text, flag_of, list};
use crate::content::{Lane, Map, MapEdge};

const PLAYER_FACTION: &str = "player";

pub fn derive(export: &Value, map: &mut Map, errors: &mut Vec<String>) {
    let links: Vec<(String, String)> = list(export, "links")
        .iter()
        .map(|link| (field_text(link, "a"), field_text(link, "b")))
        .collect();
    let nodes = list(export, "nodes");
    let mut lane_pairs = BTreeSet::new();
    match home(nodes) {
        None => errors.push(
            "no player home: mark a player-owned node as a critical asset (or an ORIGIN)".into(),
        ),
        Some(home) => {
            let parents = breadth_first(&links, &home);
            for target in targets(nodes, &home) {
                if !parents.contains_key(&target) {
                    errors.push(format!(
                        "no link path from the player home '{home}' to target '{target}'"
                    ));
                    continue;
                }
                map.lanes.push(lane(path_to(&parents, &target), &mut lane_pairs));
            }
            if map.lanes.is_empty() {
                errors.push(
                    "no lane could be derived: link the player home to an enemy target".into(),
                );
            }
        }
    }
    let mut seen = lane_pairs;
    for (a, b) in links {
        if seen.insert(pair_key(&a, &b)) {
            map.edges.push(MapEdge { node_a_id: a, node_b_id: b });
        }
    }
}

fn home(nodes: &[Value]) -> Option<String> {
    let player = |entry: &&Value| field_text(entry, "owning_faction_id") == PLAYER_FACTION;
    nodes
        .iter()
        .filter(player)
        .find(|entry| flag_of(entry, "is_critical_asset"))
        .or_else(|| {
            nodes.iter().filter(player).find(|entry| field_text(entry, "node_type") == "ORIGIN")
        })
        .map(|entry| field_text(entry, "id"))
}

/// Every other node that is a critical asset or an origin and isn't the player's.
fn targets(nodes: &[Value], home: &str) -> Vec<String> {
    nodes
        .iter()
        .filter(|entry| {
            let id = field_text(entry, "id");
            id != home
                && field_text(entry, "owning_faction_id") != PLAYER_FACTION
                && (flag_of(entry, "is_critical_asset")
                    || field_text(entry, "node_type") == "ORIGIN")
        })
        .map(|entry| field_text(entry, "id"))
        .collect()
}

/// Each node's parent on a fewest-link path from home ("" for home); ties go to the earlier
/// link.
fn breadth_first(links: &[(String, String)], home: &str) -> BTreeMap<String, String> {
    let mut neighbours: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (a, b) in links {
        neighbours.entry(a).or_default().push(b);
        neighbours.entry(b).or_default().push(a);
    }
    let mut parents = BTreeMap::from([(home.to_string(), String::new())]);
    let mut queue = VecDeque::from([home.to_string()]);
    while let Some(current) = queue.pop_front() {
        for next in neighbours.get(current.as_str()).into_iter().flatten() {
            if !parents.contains_key(*next) {
                parents.insert(next.to_string(), current.clone());
                queue.push_back(next.to_string());
            }
        }
    }
    parents
}

fn path_to(parents: &BTreeMap<String, String>, target: &str) -> Vec<String> {
    let mut path = vec![target.to_string()];
    while let Some(parent) = parents.get(&path[0]).filter(|parent| !parent.is_empty()) {
        path.insert(0, parent.clone());
    }
    path
}

/// The lane along `path`, its consecutive pairs recorded in `lane_pairs`.
fn lane(path: Vec<String>, lane_pairs: &mut BTreeSet<String>) -> Lane {
    for step in path.windows(2) {
        lane_pairs.insert(pair_key(&step[0], &step[1]));
    }
    Lane { id: format!("to_{}", path[path.len() - 1]), nodes: path, player_home_index: 0 }
}

fn pair_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}|{b}")
    } else {
        format!("{b}|{a}")
    }
}
