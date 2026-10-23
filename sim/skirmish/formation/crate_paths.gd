class_name CratePaths
extends RefCounted
## The crate's paths (`GroundPaths`, native/rust/src/paths/ and pathing.rs; spec 39 step
## 3c), which TerrainPaths, PathSearch, PathSight, SquadPathfinder, FormationPathing, and
## ClimbStamina's standing and ModeStamina's step cost are views over, and what of a
## walker, a sight, a memory and a group they pass it. Stateless, so one is shared.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const PATHS_CLASS := "GroundPaths"

static var _core: Object


## The shared one.
static func core() -> Object:
	if _core == null:
		NativeLibrary.load_for(PATHS_CLASS)
		_core = ClassDB.instantiate(PATHS_CLASS)
	return _core


## What of `walker` a path reads, then the tuning a climb reads as it stands now: [height,
## swimmer, climber, swims, climbs, stamina, speed, ground_cliff_quarters, stamina_climb,
## stamina_swim].
static func walker(path_walker: TerrainWalker) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var fields := path_walker.fields()
	fields.append_array(PackedFloat64Array([path_walker.stamina, path_walker.speed]))
	fields.append_array(
		PackedFloat64Array(
			[tuning.ground_cliff_quarters, tuning.stamina_climb, tuning.stamina_swim]
		)
	)
	return fields


## A sight's fields (PathSight.fields); empty for none: it sees everything.
static func sight(path_sight) -> PackedFloat64Array:
	return PackedFloat64Array() if path_sight == null else path_sight.fields()


## What `memory` has seen, for the crate; null where it keeps none.
static func known(memory) -> Variant:
	return null if memory == null else memory.known


## Keeps in `memory` what the crate's answer `out` says it has now seen.
static func learnt(memory, out: Dictionary) -> void:
	if memory != null:
		memory.known = out["known"]


## The group's `living` units as a look reads them, sixteen numbers each: its walker's
## seven (TerrainWalker.of_unit), its wits, its id, where it stands in the squad's store,
## then 1 and its definition's detection range and sight shares, or five 0s without one.
static func members(squad, living: Array) -> PackedFloat64Array:
	var columns: UnitColumns = squad.store.columns
	var out := PackedFloat64Array()
	# store: scattered (few: a group's units, once as it looks again or picks its pathfinder)
	for unit in living:
		var unit_walker := TerrainWalker.of_unit(unit)
		var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
		var wits := int(unit.attributes.get("wits", UnitDef.AVERAGE))
		out.append_array(unit_walker.fields())
		out.append_array(
			PackedFloat64Array([unit_walker.stamina, unit_walker.speed, wits, unit.id, at.x, at.y])
		)
		var def: UnitDef = unit.definition
		if def == null:
			out.append_array(PackedFloat64Array([0.0, 0.0, 0.0, 0.0, 0.0]))
		else:
			out.append_array(
				PackedFloat64Array(
					[1.0, def.detection_range, def.sight_ahead, def.sight_side, def.sight_behind]
				)
			)
	return out
