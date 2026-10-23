class_name MapDef
extends Resource
## Map topology and tuning definition (lanes, tick timing, suspicion ladder). See
## specs/07-data-resource-schemas.md, specs/00-scope-and-map.md, and
## specs/09-node-graph-and-lanes.md. Filled from content/maps/<name>.json by the Rust crate,
## which holds a map's rules (native/rust/src/content/map_check.rs, spec 39 step 2c) and checks
## them when it loads a map or the designer saves one.

## Phase 4 item 2: nodes: Array[NodeDef] (one implicit global lane) replaced by
## lanes: Array[LaneDef] (each an explicitly ordered node sequence) - see
## specs/09-node-graph-and-lanes.md for why.
@export var lanes: Array[LaneDef] = []

## Explicit extra connections beyond each lane's own implied path, per
## specs/11-graph-topology-edges.md. The map's full adjacency graph is the union of
## each LaneDef's own consecutive-node edges and these. Defaults empty - zero
## behavior change for any map that doesn't use branching.
@export var edges: Array[MapEdgeDef] = []

## Nodes that exist on the map but sit on no lane path - side objectives, secret
## caches, branch waypoints - per specs/16-designer-map-import.md, so importing a
## designer map never drops a node. Reachable only through edges (or not at all).
## Validated and cross-referenced exactly like lane nodes. Defaults empty.
@export var off_lane_nodes: Array[NodeDef] = []

## Additive, per specs/13-faction-def-and-garrison-unit.md. sim/'s existing ad-hoc
## String ownership is not migrated to this - that's separate, future work.
@export var faction_relations: Array[FactionRelationDef] = []

## The map's faction roster, per specs/13-faction-def-and-garrison-unit.md -
## real author-definable content, not a closed enum. Includes the player's own
## entry; "player" is authoring convention (an id like any other), not a hardcoded
## special case.
@export var factions: Array[FactionDef] = []

## Grid, tiles, roads and link routes, per specs/19-map-layout-and-objectives.md. Null
## for maps authored without the designer (p_f_F_c), which stay valid.
@export var layout: MapLayoutDef

## Each faction's loss conditions (groups of critical assets), per specs/19.
@export var loss_groups: Array[LossGroupDef] = []

@export var tick_duration_seconds: float = 0.0

## Ascending thresholds for Wary/Alarmed/Mobilized/Full Alert (Decision 5).
@export var suspicion_tier_thresholds: Array[int] = []
@export var suspicion_decay_per_tick: int = 0
