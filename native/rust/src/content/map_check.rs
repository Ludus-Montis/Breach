//! A map's rules (`MapDef.validate()` and what it called, ported in spec 39 step 2c): each
//! node, lane, edge, faction relation, loss group and the layout on its own, then every
//! reference among them and to the terrain library. The messages are GDScript's, a cell
//! written as Godot printed a `Vector2i`, `(col, row)`. The crate checks a map when it loads
//! one and when the designer saves one.

use std::collections::{BTreeMap, BTreeSet};

use super::map::{
    Channel, FactionRelation, Lane, LossGroup, Map, MapEdge, MapLayout, Node, NodeType, Subnode,
    Tile,
};
use super::TerrainLibrary;

/// Cells along a tile's edge: a tile is 64 x 64 cells (Decision 68).
pub const CELLS_PER_TILE: i64 = 64;
/// The subnode types (`SubnodeDef.TYPES`).
const SUBNODE_TYPES: [&str; 4] = ["WELL", "ORE_VEIN", "KEEP", "OBJECTIVE"];
const NO_TILE: [i64; 2] = [-1, -1];

/// A cell as Godot prints a `Vector2i`.
pub fn cell(at: &[i64; 2]) -> String {
    format!("({}, {})", at[0], at[1])
}

/// A list as Godot prints an array of cells.
fn cells(list: &[[i64; 2]]) -> String {
    format!("[{}]", list.iter().map(cell).collect::<Vec<_>>().join(", "))
}

impl Map {
    /// Every rule the map breaks, in the order GDScript checked them; none for a valid map.
    pub fn validate(&self, library: &TerrainLibrary) -> Vec<String> {
        let mut errors = Vec::new();
        if self.lanes.is_empty() {
            errors.push(format!("lanes must contain at least 1 entry, got {}", self.lanes.len()));
        }
        if self.tick_duration_seconds <= 0.0 {
            errors.push(format!(
                "tick_duration_seconds must be > 0, got {:.6}",
                self.tick_duration_seconds
            ));
        }
        let thresholds = &self.suspicion_tier_thresholds;
        if thresholds.windows(2).any(|pair| pair[1] <= pair[0]) {
            let listed: Vec<String> = thresholds.iter().map(i64::to_string).collect();
            errors.push(format!(
                "suspicion_tier_thresholds must be strictly ascending, got [{}]",
                listed.join(", ")
            ));
        }
        for lane in &self.lanes {
            errors.extend(lane.validate(self));
        }
        for node in self.off_lane_nodes() {
            for error in node.validate() {
                errors.push(format!("off-lane node '{}': {error}", node.id));
            }
        }
        let node_ids: BTreeSet<&str> = self.all_nodes().map(|node| node.id.as_str()).collect();
        let faction_ids: BTreeSet<&str> =
            self.factions.iter().map(|faction| faction.id.as_str()).collect();
        errors.extend(self.check_edges(&node_ids));
        errors.extend(self.check_node_references(&node_ids, &faction_ids));
        errors.extend(self.check_relations(&faction_ids));
        if let Some(layout) = &self.layout {
            errors.extend(layout.validate(&node_ids, library));
        }
        let critical: BTreeSet<&str> = self
            .all_nodes()
            .filter(|node| node.is_critical_asset)
            .map(|node| node.id.as_str())
            .collect();
        for group in &self.loss_groups {
            errors.extend(group.validate(&node_ids, &faction_ids, &critical));
        }
        errors.extend(self.check_footprints());
        errors
    }

    /// The node named `id`.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// The nodes on no lane, in `nodes`' order.
    pub fn off_lane_nodes(&self) -> impl Iterator<Item = &Node> {
        let on_lane: BTreeSet<&str> =
            self.lanes.iter().flat_map(|lane| lane.nodes.iter().map(String::as_str)).collect();
        self.nodes.iter().filter(move |node| !on_lane.contains(node.id.as_str()))
    }

    /// Every node as `MapDef` listed them: each lane's (a node on two lanes twice), then the
    /// off-lane ones.
    fn all_nodes(&self) -> impl Iterator<Item = &Node> {
        let on_lanes = self.lanes.iter().flat_map(|lane| lane.nodes.iter());
        on_lanes.filter_map(|id| self.node(id)).chain(self.off_lane_nodes())
    }

    fn check_edges(&self, node_ids: &BTreeSet<&str>) -> Vec<String> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for edge in &self.edges {
            errors.extend(edge.validate());
            for (end, id) in [("node_a_id", &edge.node_a_id), ("node_b_id", &edge.node_b_id)] {
                if !node_ids.contains(id.as_str()) {
                    errors.push(format!("edge references unknown {end} '{id}'"));
                }
            }
            if !seen.insert(pair_key(&edge.node_a_id, &edge.node_b_id)) {
                errors.push(format!(
                    "duplicate edge between '{}' and '{}'",
                    edge.node_a_id, edge.node_b_id
                ));
            }
        }
        errors
    }

    fn check_node_references(
        &self,
        node_ids: &BTreeSet<&str>,
        faction_ids: &BTreeSet<&str>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        for node in self.all_nodes() {
            let id = &node.id;
            let owner = &node.owning_faction_id;
            if !owner.is_empty() && !faction_ids.contains(owner.as_str()) {
                errors.push(format!(
                    "node '{id}' owning_faction_id references unknown faction id '{owner}'"
                ));
            }
            for hidden in &node.hidden_from_faction_ids {
                if !hidden.is_empty() && !faction_ids.contains(hidden.as_str()) {
                    errors.push(format!(
                        "node '{id}' hidden_from_faction_ids references unknown faction id \
                         '{hidden}'"
                    ));
                }
            }
            for unit in &node.garrison_units {
                let faction = &unit.faction_id;
                if !faction.is_empty() && !faction_ids.contains(faction.as_str()) {
                    errors.push(format!(
                        "node '{id}' garrison unit references unknown faction id '{faction}'"
                    ));
                }
                for stop in &unit.patrol_route {
                    if !node_ids.contains(stop.as_str()) {
                        errors.push(format!(
                            "node '{id}' garrison unit patrol_route references unknown node id \
                             '{stop}'"
                        ));
                    }
                }
                let target = &unit.delivery_target_id;
                if !target.is_empty() && !node_ids.contains(target.as_str()) {
                    errors.push(format!(
                        "node '{id}' garrison unit delivery_target_id references unknown node \
                         id '{target}'"
                    ));
                }
            }
        }
        errors
    }

    fn check_relations(&self, faction_ids: &BTreeSet<&str>) -> Vec<String> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for relation in &self.faction_relations {
            errors.extend(relation.validate());
            let ends = [
                ("faction_a_id", &relation.faction_a_id),
                ("faction_b_id", &relation.faction_b_id),
            ];
            for (end, id) in ends {
                if !faction_ids.contains(id.as_str()) {
                    errors.push(format!("relation references unknown {end} '{id}'"));
                }
            }
            if !seen.insert(pair_key(&relation.faction_a_id, &relation.faction_b_id)) {
                errors.push(format!(
                    "duplicate faction relation between '{}' and '{}'",
                    relation.faction_a_id, relation.faction_b_id
                ));
            }
        }
        errors
    }

    /// No tile is covered by two nodes (Decision 68).
    fn check_footprints(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut owner: BTreeMap<[i64; 2], &str> = BTreeMap::new();
        for node in self.all_nodes() {
            for at in node.covered_tiles() {
                if let Some(held) = owner.get(&at).filter(|held| **held != node.id) {
                    errors.push(format!(
                        "nodes '{held}' and '{}' share tile {}",
                        node.id,
                        cell(&at)
                    ));
                }
                owner.insert(at, &node.id);
            }
        }
        errors
    }
}

fn pair_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}|{b}")
    } else {
        format!("{b}|{a}")
    }
}

impl Lane {
    pub(super) fn validate(&self, map: &Map) -> Vec<String> {
        let mut errors = Vec::new();
        let count = self.nodes.len();
        if count < 2 {
            errors.push(format!("nodes must contain at least 2 entries, got {count}"));
        }
        if self.player_home_index < 0 || self.player_home_index >= count as i64 {
            errors.push(format!(
                "player_home_index must be a valid index into nodes, got {} (nodes size {count})",
                self.player_home_index
            ));
        }
        for node in self.nodes.iter().filter_map(|id| map.node(id)) {
            errors.extend(node.validate());
        }
        errors
    }
}

impl Node {
    /// Every tile the node covers: its footprint, else its own tile (none without one).
    pub fn covered_tiles(&self) -> Vec<[i64; 2]> {
        match (self.footprint.is_empty(), self.tile == NO_TILE) {
            (false, _) => self.footprint.clone(),
            (true, false) => vec![self.tile],
            (true, true) => Vec::new(),
        }
    }

    /// The node's own rules; its references to the rest of the map are the map's to check.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let positive = |value: i64, name: &str, kind: &str| {
            (value <= 0).then(|| format!("{name} must be > 0 for a {kind} node"))
        };
        if self.id.is_empty() {
            errors.push("id must not be empty".into());
        }
        if self.garrison < 0 {
            errors.push(format!("garrison must be >= 0, got {}", self.garrison));
        }
        if self.node_type == NodeType::Resource {
            errors.extend(positive(self.yield_food_per_tick, "yield_food_per_tick", "RESOURCE"));
            errors.extend(positive(self.decay_interval_ticks, "decay_interval_ticks", "RESOURCE"));
            if self.decay_floor_food < 0 {
                errors
                    .push(format!("decay_floor_food must be >= 0, got {}", self.decay_floor_food));
            }
            errors.extend(positive(self.ravage_yield_food, "ravage_yield_food", "RESOURCE"));
        }
        if self.garrison > 0 {
            if self.garrison_hp <= 0 {
                errors.push("garrison_hp must be > 0 when garrison > 0".into());
            }
            if self.garrison_dmg <= 0 {
                errors.push("garrison_dmg must be > 0 when garrison > 0".into());
            }
        }
        if self.node_type == NodeType::Fort {
            errors.extend(positive(self.dismantle_wood_yield, "dismantle_wood_yield", "FORT"));
            errors.extend(positive(self.dismantle_stone_yield, "dismantle_stone_yield", "FORT"));
            errors.extend(positive(self.fortify_wood_cost, "fortify_wood_cost", "FORT"));
            errors.extend(positive(self.fortify_stone_cost, "fortify_stone_cost", "FORT"));
        }
        if self.total_reserves < 0 {
            errors.push(format!("total_reserves must be >= 0, got {}", self.total_reserves));
        }
        if !self.garrison_units.is_empty() && self.garrison <= 0 {
            errors.push(format!("garrison_units requires garrison > 0, got {}", self.garrison));
        }
        for unit in &self.garrison_units {
            if unit.faction_id.is_empty() {
                errors.push("faction_id must not be empty".into());
            }
        }
        let mut seen = BTreeSet::new();
        for faction in &self.hidden_from_faction_ids {
            if faction.is_empty() {
                errors.push("hidden_from_faction_ids must not contain an empty faction id".into());
            } else if !seen.insert(faction) {
                errors.push(format!("hidden_from_faction_ids lists '{faction}' more than once"));
            }
        }
        errors.extend(self.check_footprint());
        for subnode in &self.subnodes {
            errors.extend(subnode.validate());
        }
        if self.tile != NO_TILE {
            errors.extend(Subnode::validate_in_node(
                &self.subnodes,
                &self.covered_tiles(),
                self.tile,
            ));
        }
        errors
    }

    /// The footprint includes the node's own tile and is one area, edge to edge.
    fn check_footprint(&self) -> Vec<String> {
        if self.footprint.is_empty() {
            return Vec::new();
        }
        if !self.footprint.contains(&self.tile) {
            return vec![format!(
                "node '{}': its footprint must include its own tile {}",
                self.id,
                cell(&self.tile)
            )];
        }
        let mut reached = BTreeSet::from([self.tile]);
        let mut frontier = vec![self.tile];
        while let Some(at) = frontier.pop() {
            for [dx, dy] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                let next = [at[0] + dx, at[1] + dy];
                if self.footprint.contains(&next) && reached.insert(next) {
                    frontier.push(next);
                }
            }
        }
        // A tile listed twice counts twice, as GDScript counted it.
        if reached.len() < self.footprint.len() {
            return vec![format!("node '{}': its footprint must be one connected area", self.id)];
        }
        Vec::new()
    }
}

impl Subnode {
    /// The subnode on its own; how it sits in its node is `validate_in_node`'s.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let id = &self.id;
        if id.is_empty() {
            errors.push("a subnode's id must not be empty".into());
        }
        if !SUBNODE_TYPES.contains(&self.subnode_type.as_str()) {
            errors.push(format!("subnode '{id}': unknown type '{}'", self.subnode_type));
        }
        if self.capture_cells.is_empty() {
            errors.push(format!("subnode '{id}': its capture area is empty"));
        }
        if let Some(outside) = self.capture_cells.iter().find(|at| !self.zone_cells.contains(at)) {
            errors.push(format!(
                "subnode '{id}': capture cell {} is outside its zone",
                cell(outside)
            ));
        }
        if !self.zone_cells.contains(&self.at) {
            errors
                .push(format!("subnode '{id}': its marker {} is outside its zone", cell(&self.at)));
        }
        errors
    }

    /// A node's subnodes together: unique ids, zones on the node's tiles (a cell's tile is
    /// the node's own tile plus whole tiles of cells), and no cell in two zones.
    pub fn validate_in_node(
        subnodes: &[Subnode],
        tiles: &[[i64; 2]],
        own: [i64; 2],
    ) -> Vec<String> {
        let mut errors = Vec::new();
        let mut ids = BTreeSet::new();
        let mut claimed: BTreeMap<[i64; 2], &str> = BTreeMap::new();
        for subnode in subnodes {
            if !ids.insert(&subnode.id) {
                errors.push(format!("subnode '{}' is listed more than once", subnode.id));
            }
            let mut off_tiles = false;
            for at in &subnode.zone_cells {
                let offset = Self::tile_offset(at);
                if !off_tiles && !tiles.contains(&[own[0] + offset[0], own[1] + offset[1]]) {
                    errors.push(format!(
                        "subnode '{}': zone cell {} is off the node's tiles",
                        subnode.id,
                        cell(at)
                    ));
                    off_tiles = true;
                }
                if let Some(held) = claimed.get(at).filter(|held| **held != subnode.id) {
                    errors.push(format!(
                        "subnodes '{held}' and '{}' overlap at {}",
                        subnode.id,
                        cell(at)
                    ));
                }
                claimed.insert(*at, &subnode.id);
            }
        }
        errors
    }

    /// Which tile, relative to the node's own, a local cell lies on.
    pub fn tile_offset(at: &[i64; 2]) -> [i64; 2] {
        [at[0].div_euclid(CELLS_PER_TILE), at[1].div_euclid(CELLS_PER_TILE)]
    }
}

impl MapEdge {
    pub(super) fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.node_a_id.is_empty() {
            errors.push("node_a_id must not be empty".into());
        }
        if self.node_b_id.is_empty() {
            errors.push("node_b_id must not be empty".into());
        }
        if !self.node_a_id.is_empty() && self.node_a_id == self.node_b_id {
            errors.push(format!(
                "node_a_id and node_b_id must not be the same node (self-loop), got '{}'",
                self.node_a_id
            ));
        }
        errors
    }
}

impl FactionRelation {
    pub(super) fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.faction_a_id.is_empty() {
            errors.push("faction_a_id must not be empty".into());
        }
        if self.faction_b_id.is_empty() {
            errors.push("faction_b_id must not be empty".into());
        }
        if !self.faction_a_id.is_empty() && self.faction_a_id == self.faction_b_id {
            errors.push(format!(
                "faction_a_id and faction_b_id must not be the same faction (self-relation), \
                 got '{}'",
                self.faction_a_id
            ));
        }
        errors
    }
}

impl LossGroup {
    pub(super) fn validate(
        &self,
        node_ids: &BTreeSet<&str>,
        faction_ids: &BTreeSet<&str>,
        critical: &BTreeSet<&str>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        let label = format!("loss group '{}' ({})", self.display_name, self.faction_id);
        if !faction_ids.contains(self.faction_id.as_str()) {
            errors.push(format!("{label}: unknown faction '{}'", self.faction_id));
        }
        if self.node_ids.is_empty() {
            errors.push(format!("{label} has no nodes"));
        }
        for id in &self.node_ids {
            if !node_ids.contains(id.as_str()) {
                errors.push(format!("{label}: unknown node '{id}'"));
            } else if !critical.contains(id.as_str()) {
                errors.push(format!("{label}: '{id}' is not a critical asset"));
            }
        }
        errors
    }
}

impl MapLayout {
    fn contains(&self, at: &[i64; 2]) -> bool {
        at[0] >= 0 && at[1] >= 0 && at[0] < self.cols && at[1] < self.rows
    }

    /// The first tile at `at`, if it was authored.
    pub fn tile_at(&self, at: &[i64; 2]) -> Option<&Tile> {
        self.tiles.iter().find(|tile| &tile.cell == at)
    }

    /// The terrain at `at`: its tile's, else the layout's default.
    pub fn terrain_id_at(&self, at: &[i64; 2]) -> &str {
        match self.tile_at(at) {
            Some(tile) if !tile.terrain_id.is_empty() => &tile.terrain_id,
            _ => &self.default_terrain_id,
        }
    }

    pub(super) fn validate(
        &self,
        node_ids: &BTreeSet<&str>,
        library: &TerrainLibrary,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        if self.ceiling <= 0 {
            errors.push(format!("layout ceiling must be > 0 cells, got {}", self.ceiling));
        }
        if self.cols <= 0 || self.rows <= 0 {
            errors
                .push(format!("layout cols and rows must be > 0, got {}x{}", self.cols, self.rows));
        }
        let base = library.terrains.iter().find(|terrain| terrain.id == self.default_terrain_id);
        if !base.is_some_and(|terrain| terrain.can_be_base) {
            errors.push(format!(
                "default_terrain_id '{}' must be a can_be_base terrain in the library",
                self.default_terrain_id
            ));
        }
        errors.extend(self.check_tiles(node_ids, library));
        errors.extend(self.check_roads());
        errors.extend(self.check_routes(node_ids));
        for channel in &self.channels {
            errors.extend(channel.validate(self.cols, self.rows, library));
        }
        errors
    }

    fn check_tiles(&self, node_ids: &BTreeSet<&str>, library: &TerrainLibrary) -> Vec<String> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for tile in &self.tiles {
            let at = cell(&tile.cell);
            if !self.contains(&tile.cell) {
                errors.push(format!("tile {at} is outside the {}x{} grid", self.cols, self.rows));
            }
            if !seen.insert(tile.cell) {
                errors.push(format!("duplicate tile at {at}"));
            }
            errors.extend(tile.validate());
            let terrain = |id: &str| library.terrains.iter().find(|terrain| terrain.id == id);
            if !tile.terrain_id.is_empty() && terrain(&tile.terrain_id).is_none() {
                errors.push(format!("tile {at}: unknown terrain '{}'", tile.terrain_id));
            }
            let has_feature = library.features.iter().any(|feature| feature.id == tile.feature_id);
            if !tile.feature_id.is_empty() && !has_feature {
                errors.push(format!("tile {at}: unknown feature '{}'", tile.feature_id));
            }
            if let Some(bridge) = &tile.bridge {
                let under = terrain(self.terrain_id_at(&tile.cell));
                if let Some(under) = under.filter(|under| !under.needs_bridge) {
                    errors.push(format!("bridge at {at}: '{}' doesn't need a bridge", under.id));
                }
                let controller = &bridge.drawbridge_node_id;
                if !controller.is_empty() && !node_ids.contains(controller.as_str()) {
                    errors
                        .push(format!("bridge at {at}: unknown drawbridge_node_id '{controller}'"));
                }
            }
        }
        errors
    }

    fn check_roads(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for road in &self.roads {
            let label = format!("{}-{}", cell(&road.a), cell(&road.b));
            if !self.contains(&road.a) || !self.contains(&road.b) {
                errors.push(format!("road {label} leaves the grid"));
            }
            let step = [(road.b[0] - road.a[0]).abs(), (road.b[1] - road.a[1]).abs()];
            if step == [0, 0] || step[0] > 1 || step[1] > 1 {
                errors.push(format!("road {label} must join neighbouring cells"));
            }
            if !seen.insert((road.a.min(road.b), road.a.max(road.b))) {
                errors.push(format!("duplicate road {label}"));
            }
        }
        errors
    }

    fn check_routes(&self, node_ids: &BTreeSet<&str>) -> Vec<String> {
        let mut errors = Vec::new();
        for route in &self.routes {
            let label = format!("{}-{}", route.node_a_id, route.node_b_id);
            for id in [&route.node_a_id, &route.node_b_id] {
                if !node_ids.contains(id.as_str()) {
                    errors.push(format!("route {label}: unknown node '{id}'"));
                }
            }
            if route.cells.iter().any(|at| !self.contains(at)) {
                errors.push(format!("route {label} leaves the grid"));
            }
            let gap = route.cells.windows(2).find(|pair| {
                let step = [(pair[1][0] - pair[0][0]).abs(), (pair[1][1] - pair[0][1]).abs()];
                step == [0, 0] || step[0] > 1 || step[1] > 1
            });
            if let Some(pair) = gap {
                errors.push(format!(
                    "route {label} has a gap between {} and {}",
                    cell(&pair[0]),
                    cell(&pair[1])
                ));
            }
        }
        errors
    }
}

impl Tile {
    /// The tile's own rules; the layout checks the grid and the library.
    pub(super) fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let at = cell(&self.cell);
        let capacity = [
            ("stability", self.stability),
            ("max_height", self.max_height),
            ("max_width", self.max_width),
            ("max_depth", self.max_depth),
        ];
        for (field, value) in capacity {
            if value < -1 {
                errors.push(format!(
                    "tile {at}: {field} must be -1 (terrain default) or >= 0, got {value}"
                ));
            }
        }
        if self.elevation < -1 {
            errors.push(format!(
                "tile {at}: elevation must be -1 (terrain default) or >= 0, got {}",
                self.elevation
            ));
        }
        let (upgrades, slots) = (self.upgrade_ids.len(), self.upgrade_slots);
        if upgrades as i64 > slots {
            let plural = if slots == 1 { "" } else { "s" };
            errors.push(format!("tile {at}: {upgrades} upgrades exceed {slots} slot{plural}"));
        }
        errors
    }
}

impl Channel {
    pub(super) fn validate(&self, cols: i64, rows: i64, library: &TerrainLibrary) -> Vec<String> {
        let mut errors = Vec::new();
        let at = format!("channel {}: ", cells(&self.tiles));
        if self.width < 1 {
            errors.push(format!("{at}width must be at least 1 cell, got {}", self.width));
        }
        for (index, tile) in self.tiles.iter().enumerate() {
            if tile[0] < 0 || tile[1] < 0 || tile[0] >= cols || tile[1] >= rows {
                errors.push(format!("{at}tile {} is off the grid", cell(tile)));
            }
            if index > 0 {
                let before = &self.tiles[index - 1];
                let apart = (before[0] - tile[0]).abs().max((before[1] - tile[1]).abs()) != 1;
                if apart {
                    errors.push(format!(
                        "{at}tiles {} and {} are not neighbours",
                        cell(before),
                        cell(tile)
                    ));
                }
            }
        }
        if !self.liquid_id.is_empty() {
            let liquid = library.materials.iter().find(|material| material.id == self.liquid_id);
            let flows =
                liquid.is_some_and(|liquid| liquid.traits.get("flows").is_some_and(|l| *l > 0));
            if !flows {
                errors.push(format!(
                    "{at}'{}' doesn't flow, so it can't fill a channel",
                    self.liquid_id
                ));
            }
            if self.liquid_depth < 0 || self.liquid_depth > self.depth {
                errors.push(format!(
                    "{at}liquid {} deeper than the channel's {}",
                    self.liquid_depth, self.depth
                ));
            }
        }
        errors
    }
}
