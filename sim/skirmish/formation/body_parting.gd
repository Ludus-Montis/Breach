class_name BodyParting
extends RefCounted
## UnitBodies' body-parting passes run on the native core (NativeKernels, Decision 129):
## the field brought to the bodies as they stand (as BodyFieldSync.bodies does), the passes
## run on the field's own state - its roster, radii, masses and draw order kept across ticks
## - and the bodies that moved written into the store's columns, those pushed from their
## places made loose. Since spec 39 step 8b5-xxxi it is one crate pass (`parting/pass.rs`),
## reading the squads' records there; a pair lying exactly on each other parts along a way
## the crate rolls from the battle's seed and their draws.

const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")

## Microseconds since last zeroed: [readying the call, the call, the core's own parting,
## writing back (now part of the call: 0)]. The bench reads them to split the boundary's
## cost from the work; no outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0])


## Parts the squads' bodies on `field` in one call (`parting/pass.rs`, spec 39 step
## 8b5-xxxi): the sync, the passes and the writing back are the crate's.
static func step(field: Object, squads: Array, fight_seed: int, threaded: bool) -> void:
	assert(StoreHolds.none_held(squads), StoreHolds.SPANS)
	if squads.is_empty():
		return
	var began := Time.get_ticks_usec()
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var tuning := BattleTuning.current()
	var weights := PackedFloat64Array(
		[tuning.bodies_passes, tuning.bodies_resist, tuning.bodies_brush]
	)
	var columns: UnitColumns = squads[0].store.columns
	var called := Time.get_ticks_usec()
	var stats: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.bodies_part(listed, fight_seed, weights, threaded)
	)
	spent[0] += called - began
	spent[1] += Time.get_ticks_usec() - called
	spent[2] += stats[0]
