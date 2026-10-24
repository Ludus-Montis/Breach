class_name ScrumFaceField
extends RefCounted
## FormationScrum's facing on the native core (NativeKernels, Decision 129): after the
## walk, every fighting unit turns at its turn rate towards the foe it touches, or else so
## as to arrive facing what it will do (`motion::look`) - worked out on the battle's field
## for the units the tick's slot search sent (ScrumSeekField, ctx["seek_batch"]). The
## field reads where each stands, faces and makes for from the unit store's columns, and
## what it turns by from its record, and writes each bearing into the column there (spec 35
## step 4d), as UnitMotion.turn writes it: the GDScript facing's bearings, bit for bit,
## until it retired (spec 39 step 4a), with DetMath's atan2 and acos (native/README.md).

const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## Microseconds since last zeroed: [gathering, the call, the core's own work, writing
## back]. The bench reads them; no outcome does. The gather and the write-back are the
## core's now (spec 35 step 4d), so those parts stay 0.
static var spent := PackedInt64Array([0, 0, 0, 0])


## Turns every unit of the tick's slot search (ScrumSeekField) on ctx["field"].
static func face(ctx: Dictionary) -> void:
	assert(StoreHolds.none_held(ctx["squads"]), StoreHolds.SPANS)
	var field: Object = ctx["field"]
	var seek = ctx["seek_batch"]  # ScrumSeekField.Batch
	var tuning := BattleTuning.current()
	var front := DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001
	var floats := PackedFloat64Array(
		[tuning.reach_contact, front, ctx["seconds"], ctx["pace"], tuning.scrum_crowding]
	)
	var valued: int = EntryKeys.VALUED[UnitColumns.TOWARD]
	var called := Time.get_ticks_usec()
	field.face(floats, seek.squads, seek.foe_ids, seek.members, seek.speeds, valued)
	spent[1] += Time.get_ticks_usec() - called
	spent[2] += field.usec()
