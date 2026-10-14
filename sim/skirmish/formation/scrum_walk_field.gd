class_name ScrumWalkField
extends RefCounted
## FormationScrum's walk on the native core (NativeKernels, Decision 129): every fighting
## unit steps once a tick - one making for a slot towards its next point, one with no
## slot that touches no foe back towards its place, looking where UnitShuffle says - round
## the bodies in its way (UnitSteer), over the bodies on the ground (GroundBodies) -
## worked out on the battle's field for the units the tick's slot search sent
## (ScrumSeekField, ctx["seek_batch"]): what that search made of each (its slot, if any)
## and where each stands and faces, as its sync left them on the field, with what each
## walks by, its squad's frame and the bodies lying. Where each ends and looks comes back
## and is written into its loose entry, exactly as FormationScrum._walk writes it in
## GDScript: the same bodies stepped round, the same maths in the same precision,
## DetMath's asin, sin and cos (native/README.md).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumFaceField = preload("res://sim/skirmish/formation/scrum_face_field.gd")

## What the slot search made of each unit (ScrumSeekField.Outcome).
const TOUCH := 0
const AIM := 2
const PRESS := 3

## Microseconds since last zeroed: [gathering, the call, the core's own work, writing
## back]. The bench reads them; no outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0])


## One call's per-squad and per-unit arrays.
class Batch:
	var squad_floats := PackedFloat64Array()
	var squad_anchors := PackedVector2Array()
	var motion := PackedFloat64Array()
	var frames := PackedFloat64Array()
	var lying_ats := PackedVector2Array()
	var lying_sizes := PackedFloat64Array()


## Walks every unit of the tick's slot search (ScrumSeekField) on ctx["field"].
static func walk(ctx: Dictionary) -> void:
	var field: Object = ctx["field"]
	var seek = ctx["seek_batch"]  # ScrumSeekField.Batch, with its plan's answer
	var began := Time.get_ticks_usec()
	var batch := _gather(seek.units, ctx)
	var tuning := BattleTuning.current()
	var floats := PackedFloat64Array(
		[
			ctx["seconds"],
			tuning.bodies_steer_look,
			tuning.bodies_steer_clear,
			tuning.bodies_ground_drag,
			tuning.bodies_ground_block,
		]
	)
	var called := Time.get_ticks_usec()
	var out: Array = field.walk(
		ctx["seed"],
		floats,
		seek.squads,
		batch.squad_floats,
		batch.squad_anchors,
		seek.members,
		seek.plan[0],
		seek.plan[3],
		seek.speeds,
		batch.motion,
		batch.frames,
		batch.lying_ats,
		batch.lying_sizes
	)
	var returned := Time.get_ticks_usec()
	ctx["face_batch"] = _write_back(seek, batch, out)
	spent[0] += called - began
	spent[1] += returned - called
	spent[2] += field.usec()
	spent[3] += Time.get_ticks_usec() - returned


## Each squad's pace and frame, each unit's bearing and turning (the facing's too) and,
## walking to its place, its place in its frame; the bodies lying on the ground.
static func _gather(units: Array, ctx: Dictionary) -> Batch:
	var batch := Batch.new()
	var crowding := BattleTuning.current().scrum_crowding
	var by_id := {}
	for squad in ctx["squads"]:
		by_id[squad.id] = squad
	var squads: PackedInt64Array = ctx["seek_batch"].squads
	for triple in range(0, squads.size(), 3):  # [id, foes, units] (ScrumSeekField)
		var squad: SkirmishSquad = by_id[squads[triple]]
		_frame(batch, squad, ctx["pace"] * (crowding if squad.pursuit.is_empty() else 1.0))
	var count := units.size()
	var codes: PackedByteArray = ctx["seek_batch"].plan[0]
	batch.motion.resize(3 * count)
	batch.frames.resize(4 * count)
	for member in range(count):
		var unit: SkirmishUnit = units[member]
		batch.motion[3 * member] = unit.bearing
		batch.motion[3 * member + 1] = unit.turn_rate
		batch.motion[3 * member + 2] = unit.backward_pace
		var code: int = codes[member]
		if code == TOUCH or code == AIM or code == PRESS:
			continue  # it has no place to walk to
		batch.frames[4 * member] = unit.column
		batch.frames[4 * member + 1] = unit.rank
		batch.frames[4 * member + 2] = unit.footprint_width
		batch.frames[4 * member + 3] = unit.footprint_depth
	for body in ctx["lying"]["lying"]:
		batch.lying_ats.append(body.position)
		batch.lying_sizes.append(ScrumReach.radius(body))
		batch.lying_sizes.append(float(body.footprint_width * body.footprint_depth))
	return batch


## A squad's pace and its frame, as ScrumStance.anchor and FormationScrum._walk read it.
static func _frame(batch: Batch, squad: SkirmishSquad, pace: float) -> void:
	if squad.stance.is_empty():
		batch.squad_floats.append_array([pace, squad.heading, squad.width, squad.centre_shift])
		batch.squad_anchors.append(squad.position)
	else:
		batch.squad_floats.append_array([pace, squad.stance["heading"], squad.width, 0.0])
		batch.squad_anchors.append(squad.stance["anchor"])


## Writes where each unit now stands and looks into its entry, as FormationScrum._walk;
## returns the same for the facing after it (ScrumFaceField), so it reads no entry again.
static func _write_back(seek, batch: Batch, out: Array) -> ScrumFaceField.Batch:
	var ats: PackedVector2Array = out[0]
	var towards: PackedVector2Array = out[1]
	var codes: PackedByteArray = seek.plan[0]
	var units: Array = seek.units
	var owners: Array = seek.owners
	var faced := ScrumFaceField.Batch.new()
	faced.flags.resize(units.size())
	for member in range(units.size()):
		var entry: Dictionary = owners[member].loose[units[member].id]
		var code: int = codes[member]
		if code == AIM or code == PRESS:
			entry["toward"] = entry["next"]
			# placement: the native walk, paid in FormationScrum (Decision 139)
			entry["at"] = ats[member]
			faced.flags[member] = ScrumFaceField.GOAL | ScrumFaceField.TOWARD
		elif code != TOUCH:
			entry["toward"] = towards[member]
			# placement: the native walk, paid in FormationScrum (Decision 139)
			entry["at"] = ats[member]
			entry["next"] = ats[member]
			faced.flags[member] = ScrumFaceField.TOWARD
		else:
			entry["toward"] = null
	faced.ats = ats
	faced.motion = batch.motion
	faced.nexts = seek.plan[3]
	faced.foe_ats = seek.plan[4]
	faced.towards = towards
	return faced
