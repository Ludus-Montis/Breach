//! A map (`MapDef`, spec 39 step 2c): its nodes, the lanes through them, the extra edges, the
//! factions and their relations, the loss groups, the layout (grid, tiles, roads, routes,
//! channels) and the map's tick scalars. One file a map, `content/maps/<name>.json`, which the
//! designer writes when it saves a map. The fields and their defaults are GDScript's, name for
//! name: until their last reader ports, GDScript's definitions are filled from these
//! (Decision 153).
//!
//! Every node is listed once, under `nodes`, and a lane names its nodes by id, so a node
//! shared by several lanes (the home) is one node. A node on no lane is an off-lane node
//! (`MapDef.off_lane_nodes`), in the order `nodes` lists it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::TerrainLibrary;

/// A tile or a cell, (col, row) or (x, y).
pub type Cell = [i64; 2];
/// A structure cell, (x, y, level).
pub type Cell3 = [i64; 3];

/// A map (`MapDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Map {
    /// Every node on the map, each once: those on lanes, then the off-lane ones.
    pub nodes: Vec<Node>,
    pub lanes: Vec<Lane>,
    /// Connections beyond each lane's own path (spec 11).
    pub edges: Vec<MapEdge>,
    pub faction_relations: Vec<FactionRelation>,
    /// The map's faction roster, the player's own entry included (spec 13).
    pub factions: Vec<Faction>,
    /// Grid, tiles, roads and routes (spec 19); none for a map authored without the designer.
    pub layout: Option<MapLayout>,
    /// Each faction's loss conditions (spec 19).
    pub loss_groups: Vec<LossGroup>,
    pub tick_duration_seconds: f64,
    /// Ascending thresholds for Wary, Alarmed, Mobilized and Full Alert (Decision 5).
    pub suspicion_tier_thresholds: Vec<i64>,
    pub suspicion_decay_per_tick: i64,
}

impl Default for Map {
    fn default() -> Self {
        Map {
            nodes: Vec::new(),
            lanes: Vec::new(),
            edges: Vec::new(),
            faction_relations: Vec::new(),
            factions: Vec::new(),
            layout: None,
            loss_groups: Vec::new(),
            tick_duration_seconds: 0.0,
            suspicion_tier_thresholds: Vec::new(),
            suspicion_decay_per_tick: 0,
        }
    }
}

impl Map {
    /// Whether the map keeps its references and its rules against `library`: the first
    /// missing reference (`lanes[0].nodes[1]: no f here`), else every rule it breaks.
    pub fn check(&self, library: &TerrainLibrary) -> Result<(), String> {
        if let Some((field, id)) = self.missing_reference(library) {
            return Err(format!("{field}: no {id} here"));
        }
        let broken = self.validate(library);
        if broken.is_empty() {
            Ok(())
        } else {
            Err(broken.join("; "))
        }
    }

    /// The field (`lanes[0].nodes[1]`, `layout.tiles[2].terrain_id`) and the id of the first
    /// node a lane names that isn't on the map, or the first terrain or feature the layout
    /// names that isn't in `library`.
    pub fn missing_reference(&self, library: &TerrainLibrary) -> Option<(String, String)> {
        for (at, lane) in self.lanes.iter().enumerate() {
            for (index, id) in lane.nodes.iter().enumerate() {
                if !self.nodes.iter().any(|node| &node.id == id) {
                    return Some((format!("lanes[{at}].nodes[{index}]"), id.clone()));
                }
            }
        }
        self.layout.as_ref().and_then(|layout| layout.missing_reference(library))
    }
}

/// A node (`NodeDef`). Its `node_type` and `resource_type` are named.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Node {
    pub id: String,
    /// Where it is drawn (Decision 19): the simulation never reads it.
    pub position: [f64; 2],
    pub node_type: NodeType,
    pub garrison: i64,
    pub yield_food_per_tick: i64,
    pub decay_interval_ticks: i64,
    pub decay_floor_food: i64,
    pub resource_type: ResourceType,
    pub is_inexhaustible: bool,
    pub total_reserves: i64,
    pub owning_faction_id: String,
    /// Factions that don't know it exists at the start (Decision 28).
    pub hidden_from_faction_ids: Vec<String>,
    pub is_critical_asset: bool,
    pub garrison_hp: i64,
    pub garrison_dmg: i64,
    pub garrison_units: Vec<GarrisonUnit>,
    pub ravage_yield_food: i64,
    pub dismantle_wood_yield: i64,
    pub dismantle_stone_yield: i64,
    pub fortify_wood_cost: i64,
    pub fortify_stone_cost: i64,
    pub capture_reward: String,
    /// Its structure's plan (spec 24), from its own tile; none drawn yet when absent.
    pub plan: Option<StructurePlan>,
    /// The tile it stands on; (-1, -1) for none (Decision 68).
    pub tile: Cell,
    /// The tiles it covers, its own included; empty for just its own tile.
    pub footprint: Vec<Cell>,
    /// Its objectives (Decision 72), in its local cells.
    pub subnodes: Vec<Subnode>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            id: String::new(),
            position: [0.0, 0.0],
            node_type: NodeType::Origin,
            garrison: 0,
            yield_food_per_tick: 0,
            decay_interval_ticks: 0,
            decay_floor_food: 0,
            resource_type: ResourceType::Food,
            is_inexhaustible: true,
            total_reserves: 0,
            owning_faction_id: String::new(),
            hidden_from_faction_ids: Vec::new(),
            is_critical_asset: false,
            garrison_hp: 0,
            garrison_dmg: 0,
            garrison_units: Vec::new(),
            ravage_yield_food: 0,
            dismantle_wood_yield: 0,
            dismantle_stone_yield: 0,
            fortify_wood_cost: 0,
            fortify_stone_cost: 0,
            capture_reward: String::new(),
            plan: None,
            tile: [-1, -1],
            footprint: Vec::new(),
            subnodes: Vec::new(),
        }
    }
}

/// `NodeDef.NodeType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeType {
    Origin,
    Resource,
    Fort,
    Neutral,
    Waypoint,
}

/// `NodeDef.ResourceType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceType {
    Food,
    Wood,
    Stone,
    Metal,
    Crystal,
}

/// One unit stationed at a node (`GarrisonUnitDef`, spec 13).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct GarrisonUnit {
    pub faction_id: String,
    pub can_sortie: bool,
    pub patrol_route: Vec<String>,
    pub delivery_target_id: String,
}

/// One of a node's objectives (`SubnodeDef`, Decision 72).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Subnode {
    pub id: String,
    /// WELL, ORE_VEIN, KEEP or OBJECTIVE.
    pub subnode_type: String,
    pub at: Cell,
    pub capture_cells: Vec<Cell>,
    pub zone_cells: Vec<Cell>,
}

impl Default for Subnode {
    fn default() -> Self {
        Subnode {
            id: String::new(),
            subnode_type: "OBJECTIVE".into(),
            at: [0, 0],
            capture_cells: Vec::new(),
            zone_cells: Vec::new(),
        }
    }
}

/// A structure's plan (`StructurePlanDef`, spec 24). Each keyed field is a list of
/// `[cell, value]` pairs in the order the designer drew them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct StructurePlan {
    /// Solid cells and their material.
    pub solid_cells: Vec<(Cell3, String)>,
    pub faces: Vec<StructureFace>,
    /// Ground cells removed (level < 0).
    pub dug: Vec<Cell3>,
    /// Dug cells filled again and their material (Decision 67).
    pub fills: Vec<(Cell3, String)>,
    /// Load units resting in a cell (Decision 65).
    pub loads: Vec<(Cell3, i64)>,
}

/// A wall or floor (`StructureFaceDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct StructureFace {
    pub cell: Cell3,
    pub side: Side,
    pub material_id: String,
    /// Eighths of a cell, 1 to 8.
    pub thickness: i64,
    /// The thickness grows into the neighbour across the edge.
    pub into_neighbour: bool,
}

impl Default for StructureFace {
    fn default() -> Self {
        StructureFace {
            cell: [0, 0, 0],
            side: Side::North,
            material_id: String::new(),
            thickness: 1,
            into_neighbour: false,
        }
    }
}

/// The side a face is stored on: a south or east face is its neighbour's north or west.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    North,
    West,
    Floor,
}

/// A lane (`LaneDef`): its nodes by id, in path order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Lane {
    pub id: String,
    pub nodes: Vec<String>,
    pub player_home_index: i64,
}

/// An extra connection between two nodes (`MapEdgeDef`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MapEdge {
    pub node_a_id: String,
    pub node_b_id: String,
}

/// A faction on the map (`FactionDef`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Faction {
    pub id: String,
    pub display_name: String,
}

/// How two factions stand (`FactionRelationDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct FactionRelation {
    pub faction_a_id: String,
    pub faction_b_id: String,
    pub stance: Stance,
}

impl Default for FactionRelation {
    fn default() -> Self {
        FactionRelation {
            faction_a_id: String::new(),
            faction_b_id: String::new(),
            stance: Stance::Hostile,
        }
    }
}

/// `FactionRelationDef.Stance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stance {
    Hostile,
    Neutral,
    Allied,
}

/// A group of critical assets a faction loses with (`LossGroupDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LossGroup {
    pub faction_id: String,
    pub display_name: String,
    pub rule: LossRule,
    pub node_ids: Vec<String>,
}

impl Default for LossGroup {
    fn default() -> Self {
        LossGroup {
            faction_id: String::new(),
            display_name: String::new(),
            rule: LossRule::Any,
            node_ids: Vec::new(),
        }
    }
}

/// `LossGroupDef.Rule`: lost when any, or all, of its nodes are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LossRule {
    Any,
    All,
}

/// The map's spatial layer (`MapLayoutDef`). Its terrain and feature ids are the shared
/// terrain library's (Decision 32).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MapLayout {
    pub cols: i64,
    pub rows: i64,
    /// Pixels per cell.
    pub cell_size: i64,
    pub default_terrain_id: String,
    pub tiles: Vec<Tile>,
    pub roads: Vec<RoadSegment>,
    pub routes: Vec<Route>,
    /// Ground above this height (cells) is taken to continue (Decision 56).
    pub ceiling: i64,
    /// Seeds relief, strata and liquid bodies (Decision 59).
    pub seed: i64,
    pub channels: Vec<Channel>,
}

impl Default for MapLayout {
    fn default() -> Self {
        MapLayout {
            cols: 0,
            rows: 0,
            cell_size: 64,
            default_terrain_id: String::new(),
            tiles: Vec::new(),
            roads: Vec::new(),
            routes: Vec::new(),
            ceiling: 64,
            seed: 0,
            channels: Vec::new(),
        }
    }
}

impl MapLayout {
    fn missing_reference(&self, library: &TerrainLibrary) -> Option<(String, String)> {
        let has_terrain = |id: &str| library.terrains.iter().any(|terrain| terrain.id == id);
        let has_feature = |id: &str| library.features.iter().any(|feature| feature.id == id);
        if !has_terrain(&self.default_terrain_id) {
            return Some(("layout.default_terrain_id".into(), self.default_terrain_id.clone()));
        }
        for (at, tile) in self.tiles.iter().enumerate() {
            if !tile.terrain_id.is_empty() && !has_terrain(&tile.terrain_id) {
                return Some((format!("layout.tiles[{at}].terrain_id"), tile.terrain_id.clone()));
            }
            if !tile.feature_id.is_empty() && !has_feature(&tile.feature_id) {
                return Some((format!("layout.tiles[{at}].feature_id"), tile.feature_id.clone()));
            }
        }
        None
    }
}

/// An explicitly authored tile (`TileDef`). A capacity or elevation of -1 is the terrain's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Tile {
    pub cell: Cell,
    /// Empty for the layout's default terrain.
    pub terrain_id: String,
    pub feature_id: String,
    pub bridge: Option<Bridge>,
    pub stability: i64,
    pub max_height: i64,
    pub max_width: i64,
    pub max_depth: i64,
    pub elevation: i64,
    pub upgrade_slots: i64,
    pub upgrade_ids: Vec<String>,
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            cell: [0, 0],
            terrain_id: String::new(),
            feature_id: String::new(),
            bridge: None,
            stability: -1,
            max_height: -1,
            max_width: -1,
            max_depth: -1,
            elevation: -1,
            upgrade_slots: 0,
            upgrade_ids: Vec::new(),
        }
    }
}

/// A bridge over a tile that needs one (`BridgeDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Bridge {
    pub owning_faction_id: String,
    pub hp: i64,
    pub demolishable_by_owner: bool,
    /// The node that raises it; empty for a fixed bridge.
    pub drawbridge_node_id: String,
    pub raised: bool,
}

impl Default for Bridge {
    fn default() -> Self {
        Bridge {
            owning_faction_id: String::new(),
            hp: 40,
            demolishable_by_owner: false,
            drawbridge_node_id: String::new(),
            raised: false,
        }
    }
}

/// A road between two neighbouring tiles (`RoadSegmentDef`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct RoadSegment {
    pub a: Cell,
    pub b: Cell,
}

/// A link's route over the tiles (`RouteDef`, Decision 26).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Route {
    pub node_a_id: String,
    pub node_b_id: String,
    pub cells: Vec<Cell>,
    pub cost: f64,
}

/// A river, ditch, moat or bank carved along tiles (`ChannelDef`, Decision 59).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Channel {
    pub tiles: Vec<Cell>,
    pub width: i64,
    pub depth: i64,
    pub liquid_id: String,
    pub liquid_depth: i64,
}

impl Default for Channel {
    fn default() -> Self {
        Channel { tiles: Vec::new(), width: 4, depth: 2, liquid_id: String::new(), liquid_depth: 0 }
    }
}
