class_name PathSight
extends RefCounted
## What a pathfinder can see to plan by (spec 30): a shape, not a circle - its reach ahead,
## to either side and behind, eased between them by the angle off its facing (straight
## ahead the ahead reach, square to it the side reach, straight back the behind reach,
## linearly between). A unit type sets the three as shares of its detection range
## (UnitDef.sight_ahead, sight_side, sight_behind). Within that shape, on terrain, it
## sees a point while the obscurance along its sight line stays within its budget
## (TerrainObscurance, BattleTuning.sight_budget): a few cells into a wood, less far in
## fog. Symmetric left and right, so ground mirrored is seen mirrored. A view over the
## crate's sight (native/rust/src/paths/sight.rs, spec 39 step 3c). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")

## Where it looks from (cells) and which way (a unit vector).
var origin: Vector2
var facing: Vector2
## How far (cells) it sees straight ahead, square to either side and straight back.
var ahead: float
var side: float
var behind: float
## The ground it looks over (null: nothing hides anything) and how much obscurance a sight
## line may sum before it sees no further.
var terrain: FormationTerrain
var budget: float


## The sight of a unit of `unit_def` standing at `at` on `ground`, facing `way`.
static func of(
	unit_def: UnitDef, at: Vector2, way: Vector2, ground: FormationTerrain = null
) -> PathSight:
	var reach := unit_def.detection_range
	return PathSight.new(
		at,
		way,
		reach * unit_def.sight_ahead,
		reach * unit_def.sight_side,
		reach * unit_def.sight_behind,
		ground,
		BattleTuning.current().sight_budget
	)


## How far it sees in the direction `way` (a unit vector).
func reach(way: Vector2) -> float:
	return CratePaths.core().reach(fields(), way)


## Whether it sees `point`.
func sees(point: Vector2) -> bool:
	return CratePaths.core().sees(terrain.core if terrain != null else null, fields(), point)


## What of it the crate reads: [origin x, y, facing x, y, ahead, side, behind, whether its
## ground hides anything, budget].
func fields() -> PackedFloat64Array:
	var out := PackedFloat64Array([origin.x, origin.y, facing.x, facing.y, ahead, side, behind])
	out.append_array(PackedFloat64Array([float(terrain != null), budget]))
	return out


func _init(
	at: Vector2,
	way: Vector2,
	ahead_cells: float,
	side_cells: float,
	behind_cells: float,
	ground: FormationTerrain = null,
	sight_budget: float = INF
):
	origin = at
	facing = way.normalized()
	ahead = ahead_cells
	side = side_cells
	behind = behind_cells
	terrain = ground
	budget = sight_budget
