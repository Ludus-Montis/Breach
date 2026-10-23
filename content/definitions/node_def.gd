class_name NodeDef
extends Resource
## Map node definition (Farm/Fort/Core/PlayerHome shapes). See
## specs/07-data-resource-schemas.md, specs/03-resource-nodes-and-workers.md, and
## specs/09-node-graph-and-lanes.md.

## NEUTRAL appended (Phase 4 item 2), never inserted - .tres files store the raw
## ordinal, and inserting would silently reinterpret already-authored content.
## WAYPOINT appended per specs/16-designer-map-import.md (Decision 25's candidate): a
## pure routing point with no type-specific validation.
enum NodeType { ORIGIN, RESOURCE, FORT, NEUTRAL, WAYPOINT }

## Append-only, per specs/12-node-schema-round-3.md. Matches EconomySystem._pools'
## existing five keys. Authored ahead of any node using anything but FOOD - a real
## generic-resource consumer (sim/capture_resolution.gd still hardcodes "food") is
## future work, not this field.
enum ResourceType { FOOD, WOOD, STONE, METAL, CRYSTAL }

const StructurePlanDef = preload("res://content/definitions/structure_plan_def.gd")
const SubnodeDef = preload("res://content/definitions/subnode_def.gd")

## Tile value meaning "not placed on a tile" (maps built by hand or imported before
## Decision 68): such a node covers no tiles.
const NO_TILE := Vector2i(-1, -1)

## Stable authoring identifier (Phase 4 item 2) - rendering/authoring metadata only,
## sim/ never reads it. Needed once array index is no longer globally unique across
## lanes.
@export var id: String = ""

## Rendering position (Phase 4 item 2) - authoring/rendering metadata only, per
## Decision 19: sim/ mechanics stay purely index-based and never read this field.
@export var position: Vector2 = Vector2.ZERO

@export var node_type: NodeType = NodeType.ORIGIN
@export var garrison: int = 0

## Only meaningful when node_type == RESOURCE.
@export var yield_food_per_tick: int = 0
@export var decay_interval_ticks: int = 0
@export var decay_floor_food: int = 0

## Additive, per specs/12-node-schema-round-3.md - existing Food-specific fields
## above are untouched (no rename), so sim/capture_resolution.gd's reads keep
## compiling and behaving identically. Authored, not yet read by sim/.
@export var resource_type: ResourceType = ResourceType.FOOD

## Fixes total_reserves' old 0-means-unlimited overload (round 3): 0 can't mean both
## "empty" and "infinite". Defaults true so content authored before this field
## existed keeps its effectively-unlimited behavior unchanged. Once total_reserves is
## actually consumed by a future item, intended semantics: only yield ABOVE
## decay_floor_food should deplete it when this is false - the floor stays an
## eternally-renewable baseline, leaving room for a future "maintained by skilled
## units" mechanic without another schema change.
@export var is_inexhaustible: bool = true

## The real finite pool size when is_inexhaustible is false. Ignored (unlimited)
## when is_inexhaustible is true. Authored, not yet read by sim/.
@export var total_reserves: int = 0

## Which faction controls this node - meaningful for ORIGIN-type "base" nodes, not
## hard-gated to that type. Cross-referenced against the map's faction roster in
## the crate's map rules. Per specs/13-faction-def-and-garrison-unit.md: this and
## LaneDef.player_home_index are now two independent ways to identify "the player's
## base" on the same node - a known, documented tension, not resolved here.
@export var owning_faction_id: String = ""

## Factions that do not know this node exists at map start - secret and secondary
## objectives, per specs/15-node-hidden-from-factions.md (Decision 28). Empty = known to
## every faction. Cross-referenced against the map's faction roster in the crate's map rules.
## Authored, not yet read by sim/ or presentation/; how a hidden node gets revealed
## (scouting, events) is not designed yet.
@export var hidden_from_faction_ids: Array[String] = []

## A node whose loss matters to its faction - the members of LossGroupDef groups, per
## specs/19-map-layout-and-objectives.md. Authored data; sim/ doesn't read it yet.
@export var is_critical_asset: bool = false

## Pooled blocker stats CombatResolver fights against, required when garrison > 0.
## See specs/02-lane-movement-and-combat.md, Decision 11 - `garrison` is a
## presence/count check, these are the real combat numbers.
@export var garrison_hp: int = 0
@export var garrison_dmg: int = 0

## Assignment roles, per specs/13-faction-def-and-garrison-unit.md (revised from
## specs/12 - see Decision 24): affiliation/behavior per stationed unit, not pooled
## on the node. Static defense needs no field of its own: garrison/garrison_hp/
## garrison_dmg above already work on any node_type (not type-gated), so "a
## structure can have a garrison" is already true. Empty = static only, no units
## assigned. Only meaningful with garrison > 0. Each entry's patrol_route/
## delivery_target_id/faction_id are cross-referenced against the map's real node
## ids/faction roster in the crate's map rules.
@export var garrison_units: Array[GarrisonUnitDef] = []

## CaptureResolution's capture-choice figures. See
## specs/03-resource-nodes-and-workers.md. Food/Wood/Stone-specific rather than
## generic-resource-typed, matching yield_food_per_tick's existing convention - this
## slice's only resource node produces Food and its only fort has no resource type.
@export var ravage_yield_food: int = 0  ## RESOURCE nodes only.
@export var dismantle_wood_yield: int = 0  ## FORT nodes only.
@export var dismantle_stone_yield: int = 0  ## FORT nodes only.
@export var fortify_wood_cost: int = 0  ## FORT nodes only.
@export var fortify_stone_cost: int = 0  ## FORT nodes only.

## Free-text/id placeholder for what unlocks on capture. Empty = none. No unlock
## system consumes this yet.
@export var capture_reward: String = ""

## The structure's plan of cells, faces and loads (spec 24); null = none drawn yet. Its
## cells are measured from this node's own tile (Decision 68).
@export var plan: StructurePlanDef
## The map tile (col, row) the node stands on.
@export var tile: Vector2i = NO_TILE
## The tiles the node covers, any connected shape including its own tile (Decision 68);
## empty means just its own tile.
@export var footprint: Array[Vector2i] = []
## The node's objectives (Decision 72, specs/26-subnodes.md), in its local cells. One side
## holding all of them controls the node; otherwise it is contested.
@export var subnodes: Array[SubnodeDef] = []
