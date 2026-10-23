//! A map's rules (`map_check.rs`): the scenarios of GDScript's MapDef, LaneDef, NodeDef,
//! SubnodeDef, MapLayoutDef, TileDef, ChannelDef, MapEdgeDef, FactionRelationDef,
//! GarrisonUnitDef and LossGroupDef tests, ported with the rules in spec 39 step 2c.

use std::collections::BTreeMap;

use super::*;

fn any(errors: &[String], needle: &str) -> bool {
    errors.iter().any(|error| error.contains(needle))
}

fn node(id: &str, node_type: NodeType) -> Node {
    Node { id: id.into(), node_type, ..Node::default() }
}

fn origin(id: &str) -> Node {
    node(id, NodeType::Origin)
}

fn garrisoned_fort(id: &str) -> Node {
    Node {
        garrison: 2,
        garrison_hp: 10,
        garrison_dmg: 3,
        dismantle_wood_yield: 8,
        dismantle_stone_yield: 5,
        fortify_wood_cost: 10,
        fortify_stone_cost: 6,
        ..node(id, NodeType::Fort)
    }
}

fn unit(faction_id: &str) -> GarrisonUnit {
    GarrisonUnit { faction_id: faction_id.into(), ..GarrisonUnit::default() }
}

fn faction(id: &str) -> Faction {
    Faction { id: id.into(), display_name: id.into() }
}

/// A map of `lanes`, each its id and nodes (a node in two lanes listed once), with valid
/// tick scalars.
fn map_of(lanes: Vec<(&str, Vec<Node>)>) -> Map {
    let mut map = Map {
        tick_duration_seconds: 2.0,
        suspicion_tier_thresholds: vec![25, 50, 75, 90],
        ..Map::default()
    };
    for (id, nodes) in lanes {
        let ids = nodes.iter().map(|node| node.id.clone()).collect();
        map.lanes.push(Lane { id: id.into(), nodes: ids, player_home_index: 0 });
        for node in nodes {
            if map.node(&node.id).is_none() {
                map.nodes.push(node);
            }
        }
    }
    map
}

/// One lane, "main", from `a` to `b`.
fn simple(a: &str, b: &str) -> Map {
    map_of(vec![("main", vec![origin(a), origin(b)])])
}

fn valid(map: &Map) -> Vec<String> {
    map.validate(&TerrainLibrary::default())
}

fn node_mut<'a>(map: &'a mut Map, id: &str) -> &'a mut Node {
    map.nodes.iter_mut().find(|node| node.id == id).unwrap()
}

#[test]
fn a_valid_map_has_no_errors() {
    assert!(valid(&simple("home", "core")).is_empty());
}

#[test]
fn a_map_needs_a_lane_ascending_thresholds_and_a_tick() {
    let mut map = Map { suspicion_tier_thresholds: vec![50, 30, 70, 90], ..Map::default() };
    let errors = valid(&map);
    assert!(any(&errors, "lanes must contain at least 1 entry"));
    assert!(any(&errors, "tick_duration_seconds must be > 0, got 0.000000"));
    assert!(any(
        &errors,
        "suspicion_tier_thresholds must be strictly ascending, got [50, 30, 70, 90]"
    ));
    map.suspicion_tier_thresholds = vec![25, 50];
    assert!(!any(&valid(&map), "suspicion"));
}

#[test]
fn a_lanes_errors_are_the_maps() {
    let mut map = simple("home", "core");
    map.lanes[0].player_home_index = 9;
    assert!(any(
        &valid(&map),
        "player_home_index must be a valid index into nodes, got 9 (nodes size 2)"
    ));
    map.lanes[0].player_home_index = -1;
    assert!(any(&valid(&map), "player_home_index"));
    map.lanes[0].player_home_index = 0;
    map.lanes[0].nodes.truncate(1);
    assert!(any(&valid(&map), "nodes must contain at least 2 entries, got 1"));
}

#[test]
fn a_lanes_node_errors_are_the_maps() {
    let mut map = simple("home", "bad");
    node_mut(&mut map, "bad").garrison = -1;
    assert!(any(&valid(&map), "garrison must be >= 0, got -1"));
}

#[test]
fn an_edge_to_a_node_that_isnt_there_is_an_error() {
    let mut map = simple("p", "f");
    map.edges.push(MapEdge { node_a_id: "p".into(), node_b_id: "ghost".into() });
    assert!(any(&valid(&map), "edge references unknown node_b_id 'ghost'"));
}

#[test]
fn a_duplicate_edge_either_way_round_is_an_error() {
    let mut map = simple("p", "f");
    map.edges.push(MapEdge { node_a_id: "p".into(), node_b_id: "f".into() });
    map.edges.push(MapEdge { node_a_id: "f".into(), node_b_id: "p".into() });
    assert!(any(&valid(&map), "duplicate edge between 'f' and 'p'"));
}

#[test]
fn an_edge_across_two_lanes_is_valid() {
    let mut map = map_of(vec![
        ("a", vec![origin("a_home"), origin("a_core")]),
        ("b", vec![origin("b_home"), origin("b_core")]),
    ]);
    map.edges.push(MapEdge { node_a_id: "a_home".into(), node_b_id: "b_home".into() });
    assert!(valid(&map).is_empty());
}

#[test]
fn an_edge_needs_two_different_ends() {
    let edge = |a: &str, b: &str| MapEdge { node_a_id: a.into(), node_b_id: b.into() };
    assert!(edge("p", "f").validate().is_empty());
    assert!(any(&edge("", "f").validate(), "node_a_id must not be empty"));
    assert!(any(&edge("p", "").validate(), "node_b_id must not be empty"));
    assert!(any(&edge("p", "p").validate(), "self-loop"));
}

#[test]
fn two_nodes_cannot_share_a_tile() {
    let mut map = simple("home", "core");
    node_mut(&mut map, "home").tile = [0, 0];
    let core = node_mut(&mut map, "core");
    core.tile = [2, 0];
    core.footprint = vec![[2, 0], [1, 0], [0, 0]];
    assert!(any(&valid(&map), "nodes 'home' and 'core' share tile (0, 0)"));
}

#[test]
fn off_lane_nodes_are_checked_and_edges_may_reach_them() {
    let mut map = simple("home", "core");
    map.nodes.push(node("cache", NodeType::Waypoint));
    map.edges.push(MapEdge { node_a_id: "core".into(), node_b_id: "cache".into() });
    assert!(valid(&map).is_empty());
    let cache = node_mut(&mut map, "cache");
    cache.node_type = NodeType::Resource;
    cache.owning_faction_id = "ghost_faction".into();
    let errors = valid(&map);
    assert!(any(&errors, "off-lane node 'cache': yield_food_per_tick must be > 0"));
    assert!(any(
        &errors,
        "node 'cache' owning_faction_id references unknown faction id 'ghost_faction'"
    ));
}

#[test]
fn a_garrison_units_patrol_delivery_and_faction_must_be_on_the_map() {
    let mut fort = garrisoned_fort("F");
    fort.garrison_units = vec![GarrisonUnit {
        patrol_route: vec!["p".into(), "ghost".into()],
        delivery_target_id: "nowhere".into(),
        ..unit("the_kingdom")
    }];
    let mut map = map_of(vec![("main", vec![origin("p"), fort])]);
    map.factions = vec![faction("the_kingdom")];
    let errors = valid(&map);
    assert!(any(&errors, "patrol_route references unknown node id 'ghost'"));
    assert!(any(&errors, "delivery_target_id references unknown node id 'nowhere'"));
    map.factions.clear();
    assert!(any(&valid(&map), "garrison unit references unknown faction id 'the_kingdom'"));
}

#[test]
fn hidden_from_must_name_a_faction_on_the_map() {
    let mut map = simple("home", "core");
    node_mut(&mut map, "core").hidden_from_faction_ids = vec!["the_kingdom".into()];
    map.factions = vec![faction("player")];
    assert!(any(
        &valid(&map),
        "hidden_from_faction_ids references unknown faction id 'the_kingdom'"
    ));
    map.factions.push(faction("the_kingdom"));
    assert!(valid(&map).is_empty());
}

#[test]
fn faction_relations_are_checked_on_their_own_and_against_the_roster() {
    let relation = |a: &str, b: &str| FactionRelation {
        faction_a_id: a.into(),
        faction_b_id: b.into(),
        ..FactionRelation::default()
    };
    assert!(relation("player", "the_kingdom").validate().is_empty());
    assert!(any(&relation("player", "player").validate(), "self-relation"));
    assert!(any(&relation("", "the_kingdom").validate(), "faction_a_id must not be empty"));
    assert!(any(&relation("player", "").validate(), "faction_b_id must not be empty"));
    let mut map = simple("home", "core");
    map.factions = vec![faction("player")];
    map.faction_relations = vec![relation("player", "the_kingdom")];
    assert!(any(&valid(&map), "relation references unknown faction_b_id 'the_kingdom'"));
    map.factions.push(faction("the_kingdom"));
    assert!(valid(&map).is_empty());
    map.faction_relations.push(relation("the_kingdom", "player"));
    assert!(any(&valid(&map), "duplicate faction relation"));
}

#[test]
fn loss_groups_hold_critical_assets_of_a_faction_on_the_map() {
    let mut map = simple("home", "core");
    node_mut(&mut map, "home").is_critical_asset = true;
    map.factions = vec![faction("player")];
    let group = |faction: &str, ids: &[&str]| LossGroup {
        faction_id: faction.into(),
        display_name: "Home".into(),
        node_ids: ids.iter().map(|id| id.to_string()).collect(),
        ..LossGroup::default()
    };
    map.loss_groups = vec![group("player", &["home"])];
    assert!(valid(&map).is_empty());
    map.loss_groups = vec![
        group("player", &[]),
        group("player", &["ghost", "core"]),
        group("kingdom", &["home"]),
    ];
    let errors = valid(&map);
    assert!(any(&errors, "loss group 'Home' (player) has no nodes"));
    assert!(any(&errors, "unknown node 'ghost'"));
    assert!(any(&errors, "'core' is not a critical asset"));
    assert!(any(&errors, "unknown faction 'kingdom'"));
}

#[test]
fn a_node_with_only_its_type_set_is_valid() {
    for node_type in [NodeType::Origin, NodeType::Neutral, NodeType::Waypoint] {
        assert!(node("n", node_type).validate().is_empty(), "{node_type:?}");
    }
    assert!(any(&node("", NodeType::Origin).validate(), "id must not be empty"));
}

#[test]
fn a_resource_node_needs_its_yields() {
    let errors = node("farm", NodeType::Resource).validate();
    assert!(any(&errors, "yield_food_per_tick must be > 0 for a RESOURCE node"));
    assert!(any(&errors, "decay_interval_ticks must be > 0 for a RESOURCE node"));
    assert!(any(&errors, "ravage_yield_food must be > 0 for a RESOURCE node"));
    let farm = Node {
        yield_food_per_tick: 6,
        decay_interval_ticks: 5,
        ravage_yield_food: 40,
        ..node("farm", NodeType::Resource)
    };
    assert!(farm.validate().is_empty());
    assert!(farm.is_inexhaustible && farm.resource_type == ResourceType::Food);
}

#[test]
fn a_fort_needs_its_figures_and_a_garrison_its_stats() {
    let fort = Node { garrison: 3, ..node("fort", NodeType::Fort) };
    let errors = fort.validate();
    assert!(any(&errors, "garrison_hp must be > 0 when garrison > 0"));
    assert!(any(&errors, "garrison_dmg must be > 0 when garrison > 0"));
    for field in
        ["dismantle_wood_yield", "dismantle_stone_yield", "fortify_wood_cost", "fortify_stone_cost"]
    {
        assert!(any(&errors, &format!("{field} must be > 0 for a FORT node")), "{field}");
    }
    assert!(garrisoned_fort("fort").validate().is_empty());
}

#[test]
fn a_nodes_counts_and_garrison_units_are_checked() {
    let reserves = Node { total_reserves: -1, ..origin("home") };
    assert!(any(&reserves.validate(), "total_reserves must be >= 0, got -1"));
    let unmanned = Node { garrison_units: vec![unit("the_kingdom")], ..origin("home") };
    assert!(any(&unmanned.validate(), "garrison_units requires garrison > 0, got 0"));
    let manned = Node { garrison_units: vec![unit("the_kingdom")], ..garrisoned_fort("fort") };
    assert!(manned.validate().is_empty());
    let factionless = Node { garrison_units: vec![unit("")], ..garrisoned_fort("fort") };
    assert!(any(&factionless.validate(), "faction_id must not be empty"));
}

#[test]
fn hidden_from_ids_are_neither_empty_nor_repeated() {
    let hidden = |ids: &[&str]| Node {
        hidden_from_faction_ids: ids.iter().map(|id| id.to_string()).collect(),
        ..origin("home")
    };
    assert!(hidden(&[]).validate().is_empty());
    assert!(any(&hidden(&[""]).validate(), "must not contain an empty faction id"));
    assert!(any(
        &hidden(&["the_kingdom", "the_kingdom"]).validate(),
        "lists 'the_kingdom' more than once"
    ));
}

fn node_at(tile: [i64; 2], footprint: &[[i64; 2]]) -> Node {
    Node { tile, footprint: footprint.to_vec(), ..node("n", NodeType::Waypoint) }
}

#[test]
fn a_node_covers_its_footprint_else_its_tile_else_nothing() {
    assert!(Node::default().covered_tiles().is_empty());
    assert_eq!(node_at([2, 3], &[]).covered_tiles(), [[2, 3]]);
    let l_shape = node_at([0, 0], &[[0, 0], [1, 0], [1, 1]]);
    assert!(l_shape.validate().is_empty());
    assert_eq!(l_shape.covered_tiles().len(), 3);
}

#[test]
fn a_footprint_holds_the_nodes_tile_and_is_one_area() {
    assert!(any(&node_at([0, 0], &[[5, 5]]).validate(), "must include its own tile (0, 0)"));
    assert!(any(&node_at([0, 0], &[[0, 0], [2, 0]]).validate(), "one connected area"));
}

fn square(x0: i64, y0: i64, size: i64) -> Vec<[i64; 2]> {
    (x0..x0 + size).flat_map(|x| (y0..y0 + size).map(move |y| [x, y])).collect()
}

fn subnode(id: &str, at: [i64; 2], capture: Vec<[i64; 2]>, zone: Vec<[i64; 2]>) -> Subnode {
    Subnode {
        id: id.into(),
        subnode_type: "WELL".into(),
        at,
        capture_cells: capture,
        zone_cells: zone,
    }
}

fn well(id: &str, x0: i64, y0: i64) -> Subnode {
    subnode(id, [x0 + 2, y0 + 2], square(x0 + 1, y0 + 1, 3), square(x0, y0, 5))
}

fn node_with(subnodes: Vec<Subnode>, footprint: &[[i64; 2]]) -> Node {
    Node { subnodes, ..node_at([3, 2], footprint) }
}

#[test]
fn a_subnode_needs_a_capture_area_marker_and_type_in_its_zone() {
    assert!(node_with(vec![well("well_1", 10, 10)], &[]).validate().is_empty());
    assert!(any(
        &subnode("w", [1, 1], vec![], square(0, 0, 3)).validate(),
        "capture area is empty"
    ));
    assert!(any(
        &subnode("w", [1, 1], vec![[9, 9]], square(0, 0, 3)).validate(),
        "capture cell (9, 9) is outside its zone"
    ));
    assert!(any(
        &subnode("w", [20, 20], vec![[1, 1]], square(0, 0, 3)).validate(),
        "its marker (20, 20) is outside its zone"
    ));
    let volcano = Subnode { subnode_type: "VOLCANO".into(), ..well("w", 0, 0) };
    assert!(any(&volcano.validate(), "unknown type 'VOLCANO'"));
}

#[test]
fn a_nodes_subnodes_are_unique_apart_and_on_its_tiles() {
    let overlapping = node_with(vec![well("well_1", 10, 10), well("well_2", 13, 10)], &[]);
    assert!(any(&overlapping.validate(), "subnodes 'well_1' and 'well_2' overlap at"));
    let repeated = node_with(vec![well("well_1", 0, 0), well("well_1", 20, 20)], &[]);
    assert!(any(&repeated.validate(), "subnode 'well_1' is listed more than once"));
    assert!(any(&node_with(vec![well("well_1", 62, 10)], &[]).validate(), "off the node's tiles"));
    let across = node_with(vec![well("well_1", 62, 10)], &[[3, 2], [4, 2]]);
    assert!(across.validate().is_empty());
    let up_left = node_with(vec![well("well_1", -3, -3)], &[[3, 2], [2, 2]]);
    assert!(any(&up_left.validate(), "off the node's tiles")); // (2, 1) isn't covered
}

fn terrain(id: &str, can_be_base: bool, needs_bridge: bool) -> Terrain {
    Terrain { id: id.into(), can_be_base, needs_bridge, ..Terrain::default() }
}

/// FIELDS and FOREST to build on, WATER that needs a bridge, the ORE feature, and WATER
/// that flows and ROCK that doesn't.
fn library() -> TerrainLibrary {
    TerrainLibrary {
        terrains: vec![
            terrain("FIELDS", true, false),
            terrain("FOREST", true, false),
            terrain("WATER", false, true),
        ],
        features: vec![TerrainFeature { id: "ORE".into(), ..TerrainFeature::default() }],
        materials: vec![
            Material {
                id: "WATER".into(),
                traits: BTreeMap::from([("flows".into(), 3)]),
                ..Material::default()
            },
            Material { id: "ROCK".into(), ..Material::default() },
        ],
        ..TerrainLibrary::default()
    }
}

fn tile(col: i64, row: i64, terrain_id: &str) -> Tile {
    Tile { cell: [col, row], terrain_id: terrain_id.into(), ..Tile::default() }
}

/// A 6 x 4 layout on FIELDS, on a map whose nodes are "a" and "b".
fn layout_map(layout: MapLayout) -> Map {
    Map {
        layout: Some(MapLayout { cols: 6, rows: 4, default_terrain_id: "FIELDS".into(), ..layout }),
        ..simple("a", "b")
    }
}

fn layout_errors(layout: MapLayout) -> Vec<String> {
    layout_map(layout).validate(&library())
}

#[test]
fn a_plain_layout_is_valid() {
    let tiles = vec![tile(1, 1, "FOREST"), tile(2, 1, "")];
    assert!(layout_errors(MapLayout { tiles, ..MapLayout::default() }).is_empty());
}

#[test]
fn the_grid_ceiling_and_default_terrain_are_checked() {
    let mut map = layout_map(MapLayout::default());
    let layout = map.layout.as_mut().unwrap();
    layout.cols = 0;
    layout.ceiling = 0;
    layout.default_terrain_id = "WATER".into();
    let errors = map.validate(&library());
    assert!(any(&errors, "layout cols and rows must be > 0, got 0x4"));
    assert!(any(&errors, "layout ceiling must be > 0 cells, got 0"));
    assert!(any(&errors, "default_terrain_id 'WATER' must be a can_be_base terrain"));
}

#[test]
fn tiles_are_on_the_grid_once_and_name_the_librarys_terrain_and_features() {
    let gold = Tile { feature_id: "GOLD".into(), ..tile(2, 2, "") };
    let tiles = vec![tile(9, 1, ""), tile(1, 1, ""), tile(1, 1, ""), tile(3, 3, "LAVA"), gold];
    let errors = layout_errors(MapLayout { tiles, ..MapLayout::default() });
    assert!(any(&errors, "tile (9, 1) is outside the 6x4 grid"));
    assert!(any(&errors, "duplicate tile at (1, 1)"));
    assert!(any(&errors, "tile (3, 3): unknown terrain 'LAVA'"));
    assert!(any(&errors, "tile (2, 2): unknown feature 'GOLD'"));
}

#[test]
fn a_tiles_overrides_elevation_and_upgrades_are_checked() {
    let tiles = vec![Tile {
        stability: -2,
        elevation: -2,
        upgrade_slots: 1,
        upgrade_ids: vec!["BARRACKS".into(), "MILL".into()],
        ..tile(1, 1, "")
    }];
    let errors = layout_errors(MapLayout { tiles, ..MapLayout::default() });
    assert!(any(&errors, "tile (1, 1): stability must be -1 (terrain default) or >= 0, got -2"));
    assert!(any(&errors, "tile (1, 1): elevation must be -1"));
    assert!(any(&errors, "2 upgrades exceed 1 slot"));
}

#[test]
fn bridges_need_terrain_that_needs_one_and_a_real_drawbridge_node() {
    let on_fields = Tile { bridge: Some(Bridge::default()), ..tile(1, 1, "") };
    let raised = Bridge { drawbridge_node_id: "nobody".into(), ..Bridge::default() };
    let drawbridge = Tile { bridge: Some(raised), ..tile(2, 1, "WATER") };
    let errors =
        layout_errors(MapLayout { tiles: vec![on_fields, drawbridge], ..MapLayout::default() });
    assert!(any(&errors, "bridge at (1, 1): 'FIELDS' doesn't need a bridge"));
    assert!(any(&errors, "bridge at (2, 1): unknown drawbridge_node_id 'nobody'"));
    assert_eq!(errors.len(), 2);
}

#[test]
fn roads_join_neighbouring_cells_on_the_grid_once() {
    let road = |a: [i64; 2], b: [i64; 2]| RoadSegment { a, b };
    let roads = vec![
        road([0, 0], [1, 1]),
        road([1, 1], [0, 0]),
        road([0, 0], [2, 0]),
        road([5, 3], [6, 3]),
    ];
    let errors = layout_errors(MapLayout { roads, ..MapLayout::default() });
    assert!(any(&errors, "duplicate road (1, 1)-(0, 0)"));
    assert!(any(&errors, "road (0, 0)-(2, 0) must join neighbouring cells"));
    assert!(any(&errors, "road (5, 3)-(6, 3) leaves the grid"));
}

#[test]
fn routes_are_unbroken_on_the_grid_between_real_nodes() {
    let route = |cells: Vec<[i64; 2]>, b: &str| Route {
        node_a_id: "a".into(),
        node_b_id: b.into(),
        cells,
        ..Route::default()
    };
    let routes = vec![
        route(vec![[0, 0], [1, 1], [2, 1]], "b"),
        route(vec![[0, 0], [2, 0]], "b"),
        route(vec![[0, 0]], "ghost"),
    ];
    let errors = layout_errors(MapLayout { routes, ..MapLayout::default() });
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(any(&errors, "route a-b has a gap between (0, 0) and (2, 0)"));
    assert!(any(&errors, "route a-ghost: unknown node 'ghost'"));
}

fn river() -> Channel {
    Channel {
        tiles: vec![[0, 0], [1, 1], [2, 1]],
        width: 4,
        depth: 3,
        liquid_id: "WATER".into(),
        liquid_depth: 2,
    }
}

fn channel_errors(channel: Channel, cols: i64) -> Vec<String> {
    channel.validate(cols, 2, &library())
}

#[test]
fn a_river_along_neighbouring_tiles_is_valid() {
    assert!(channel_errors(river(), 3).is_empty());
}

#[test]
fn a_channels_tiles_width_and_liquid_are_checked() {
    assert!(any(
        &channel_errors(river(), 2),
        "channel [(0, 0), (1, 1), (2, 1)]: tile (2, 1) is off the grid"
    ));
    let apart = Channel { tiles: vec![[0, 0], [2, 0]], ..river() };
    assert!(any(&channel_errors(apart, 3), "tiles (0, 0) and (2, 0) are not neighbours"));
    let rock = Channel { liquid_id: "ROCK".into(), ..river() };
    assert!(any(&channel_errors(rock, 3), "'ROCK' doesn't flow, so it can't fill a channel"));
    let deep = Channel { liquid_depth: 4, ..river() };
    assert!(any(&channel_errors(deep, 3), "liquid 4 deeper than the channel's 3"));
    let narrow = Channel { width: 0, ..river() };
    assert!(any(&channel_errors(narrow, 3), "width must be at least 1 cell, got 0"));
}
