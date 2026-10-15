class_name FormationSight
extends RefCounted
## What a formation can detect (Decision 87, spec 27 round 2): another squad is detected
## if any of its living units is within the best detection range among the formation's
## living units, measured in cells between unit centres. Nothing else is known to it - no
## formation is telepathic. With terrain, a sight line that passes through more than
## MAX_SCREEN cells that block sight (a wood) is blocked: units at a wood's edge see out and
## are seen, deeper in they are hidden (Decision 87). Light comes later. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## How many sight-blocking cells a sight line may cross and still see.
const MAX_SCREEN := 2
## Cells added to a unit's range in finding the buckets to look in: far above float
## rounding, so no unit in range is missed.
const MARGIN := 0.01


## Whether any pair is in range and in sight is the same whichever is tried first, so each
## unit looks only at the other's units in the grid buckets its range can reach (SquadMemo).
## `memo`, the phase's, keeps the other's grid for every squad looking at it.
static func detects(
	squad: SkirmishSquad,
	other: SkirmishSquad,
	terrain: FormationTerrain = null,
	memo: SquadMemo = null
) -> bool:
	var grid := (memo if memo != null else SquadMemo.new()).sighted(other)
	var cells: Dictionary = grid["cells"]
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		var here := at[unit.slot]
		var reach := Vector2.ONE * (unit.detection + MARGIN)
		var low: Vector2i = Vector2i(((here - reach) / SquadMemo.SIGHT_CELL).floor())
		var high: Vector2i = Vector2i(((here + reach) / SquadMemo.SIGHT_CELL).floor())
		low = low.max(grid["low"])
		high = high.min(grid["high"])
		for y in range(low.y, high.y + 1):
			for x in range(low.x, high.x + 1):
				for seen in cells.get(Vector2i(x, y), []):
					if here.distance_to(seen) > unit.detection:
						continue
					if terrain == null or clear(terrain, here, seen):
						return true
	return false


## True if the sight line between two points crosses at most MAX_SCREEN blocking cells.
static func clear(terrain: FormationTerrain, from: Vector2, to: Vector2) -> bool:
	var screened := {}
	var steps := maxi(1, ceili(from.distance_to(to) * 2.0))
	for index in range(steps + 1):
		var at := from.lerp(to, float(index) / steps)
		if terrain.blocks_sight(at):
			screened[Vector2i(floori(at.x), floori(at.y))] = true
			if screened.size() > MAX_SCREEN:
				return false
	return true
