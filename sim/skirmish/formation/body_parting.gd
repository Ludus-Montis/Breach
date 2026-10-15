class_name BodyParting
extends RefCounted
## UnitBodies' body-parting passes run on the native core (NativeKernels, Decision 129):
## the field brought to the bodies as they stand (BodyFieldSync), the passes run on the
## field's own state - its roster, radii, masses and draw order kept across ticks - and only
## the bodies that moved written back, exactly as UnitBodies' own _move would have left
## them, so the battle can't tell which ran. The core mirrors the GDScript's precision and
## order (Vector2 maths in 32-bit floats, scalars in 64-bit, pairs in order); a pair lying
## exactly on each other asks `way` (UnitBodies.part_way) which way it parts.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

## Microseconds since last zeroed: [syncing the field, the call, the core's own work,
## writing back]. The bench reads them to split the boundary's cost from the work; no
## outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0])


## Parts the squads' bodies on `field`; `way.call(draw, other draw)` is UnitBodies.part_way.
static func step(
	field: Object, squads: Array, fight_seed: int, way: Callable, threaded: bool
) -> void:
	var began := Time.get_ticks_usec()
	var synced := BodyFieldSync.bodies(field, squads, fight_seed)
	var called := Time.get_ticks_usec()
	var tuning := BattleTuning.current()
	var weights := PackedFloat64Array(
		[tuning.bodies_passes, tuning.bodies_resist, tuning.bodies_brush]
	)
	var out: Array = field.part(weights, way, threaded)
	var returned := Time.get_ticks_usec()
	_write_back(synced[0], synced[1], out)
	spent[0] += called - began
	spent[1] += returned - called
	spent[2] += out[4][0]
	spent[3] += Time.get_ticks_usec() - returned


## Writes the core's [moved, points, nexts, loosened, stats] back: the units it made loose
## first (in the order the passes did), then every moved body's point (one left in its
## frame: where it stands).
static func _write_back(units: Array, owners: Array, out: Array) -> void:
	var moved: PackedInt32Array = out[0]
	var points: PackedVector2Array = out[1]
	var nexts: PackedVector2Array = out[2]
	if units.is_empty():
		return
	var columns: UnitColumns = owners[0].store.columns
	for index in out[3]:
		var unit: SkirmishUnit = units[index]
		var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
		SquadEntries.loosen(owners[index], unit, {"unit": unit, "goal": null, "next": at}, at)
	for k in range(moved.size()):
		var squad: SkirmishSquad = owners[moved[k]]
		var unit: SkirmishUnit = units[moved[k]]
		if squad.fleeing.has(unit.id):
			# placement: parting clears an overlap (Decision 139)
			columns.put(unit.slot, UnitColumns.FLEE_OFFSET, points[k])
			continue
		if not squad.loose.has(unit.id):
			# placement: parting clears an overlap (Decision 139)
			columns.put(unit.slot, UnitColumns.POSITION, points[k])
			continue
		# placement: parting clears an overlap (Decision 139)
		columns.put(unit.slot, UnitColumns.LOOSE_AT, points[k])
		squad.loose[unit.id]["next"] = nexts[k]
