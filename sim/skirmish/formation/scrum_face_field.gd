class_name ScrumFaceField
extends RefCounted
## FormationScrum's facing on the native core (NativeKernels, Decision 129): after the
## walk, every fighting unit turns at its turn rate towards the foe it touches, or else so
## as to arrive facing what it will do (UnitShuffle) - worked out on the battle's field
## for the units the tick's slot search sent (ScrumSeekField, ctx["seek_batch"]), with
## where each now stands, what it turns by and what its entry says it makes for. The
## bearings come back and are written to the units, exactly as UnitMotion.turn writes
## them in GDScript: the same foes, the same maths in the same precision, DetMath's
## atan2 and acos (native/README.md).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## A unit's flags, as the core reads them: it makes for a slot; its walk chose a point.
const GOAL := 1
const TOWARD := 2

## Microseconds since last zeroed: [gathering, the call, the core's own work, writing
## back]. The bench reads them; no outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0])


## One call's per-unit arrays.
class Batch:
	var ats := PackedVector2Array()
	var motion := PackedFloat64Array()
	var flags := PackedByteArray()
	var nexts := PackedVector2Array()
	var foe_ats := PackedVector2Array()
	var towards := PackedVector2Array()


## Turns every unit of the tick's slot search (ScrumSeekField) on ctx["field"].
static func face(ctx: Dictionary) -> void:
	var field: Object = ctx["field"]
	var seek = ctx["seek_batch"]  # ScrumSeekField.Batch
	var began := Time.get_ticks_usec()
	var batch: Batch = ctx.get("face_batch")  # the walk's, on the field (ScrumWalkField)
	if batch == null:
		batch = _gather(seek.units, seek.owners)
	var tuning := BattleTuning.current()
	var front := DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001
	var floats := PackedFloat64Array(
		[tuning.reach_contact, front, ctx["seconds"], ctx["pace"], tuning.scrum_crowding]
	)
	var called := Time.get_ticks_usec()
	var bearings: PackedFloat64Array = field.face(
		floats,
		seek.squads,
		seek.foe_ids,
		seek.members,
		batch.ats,
		batch.motion,
		seek.speeds,
		batch.flags,
		batch.nexts,
		batch.foe_ats,
		batch.towards
	)
	var returned := Time.get_ticks_usec()
	var units: Array = seek.units
	for member in range(bearings.size()):
		units[member].bearing = bearings[member]
	spent[0] += called - began
	spent[1] += returned - called
	spent[2] += field.usec()
	spent[3] += Time.get_ticks_usec() - returned


## Where each unit stands now, how it turns, and what its loose entry makes for.
static func _gather(units: Array, owners: Array) -> Batch:
	var batch := Batch.new()
	if units.is_empty():
		return batch
	var columns: UnitColumns = owners[0].store.columns
	var loose_at: PackedVector2Array = columns.column(UnitColumns.LOOSE_AT)
	var goal_slots: PackedInt64Array = columns.column(UnitColumns.GOAL_SLOT)
	var nexts: PackedVector2Array = columns.column(UnitColumns.NEXT)
	var foe_ats: PackedVector2Array = columns.column(UnitColumns.FOE_AT)
	var towards: PackedVector2Array = columns.column(UnitColumns.TOWARD)
	var keys: PackedInt32Array = columns.column(UnitColumns.KEYS)
	var toward_bit: int = EntryKeys.VALUED[UnitColumns.TOWARD]
	for member in range(units.size()):
		var unit: SkirmishUnit = units[member]
		var slot := unit.slot
		batch.ats.append(loose_at[slot])
		batch.motion.append(unit.bearing)
		batch.motion.append(unit.turn_rate)
		batch.motion.append(unit.backward_pace)
		var flags := 0
		if goal_slots[slot] >= 0:
			flags = GOAL
			batch.nexts.append(nexts[slot])
			batch.foe_ats.append(foe_ats[slot])
		else:
			batch.nexts.append(Vector2.ZERO)
			batch.foe_ats.append(Vector2.ZERO)
		if keys[slot] & toward_bit:
			flags |= TOWARD
			batch.towards.append(towards[slot])
		else:
			batch.towards.append(Vector2.ZERO)
		batch.flags.append(flags)
	return batch
