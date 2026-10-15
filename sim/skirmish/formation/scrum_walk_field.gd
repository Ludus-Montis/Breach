class_name ScrumWalkField
extends RefCounted
## FormationScrum's walk on the native core (NativeKernels, Decision 129): every fighting
## unit steps once a tick - one making for a slot towards its next point, one with no
## slot that touches no foe back towards its place, looking where UnitShuffle says - round
## the bodies in its way (UnitSteer), over the bodies on the ground (GroundBodies) -
## worked out on the battle's field for the units the tick's slot search sent
## (ScrumSeekField, ctx["seek_batch"]): what that search made of each (its slot, if any),
## each squad's frame and pace, and the bodies lying. The field reads where each unit
## stands, faces and keeps its place (rank and column) from the unit store's columns, and
## what it turns and walks back by from its record, and writes where each ends and looks
## into its loose entry there (spec 35 step 4c), exactly as FormationScrum._walk writes it
## in GDScript: the same bodies stepped round, the same maths in the same precision,
## DetMath's asin, sin and cos (native/README.md).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## What the slot search made of each unit (ScrumSeekField.Outcome).
const TOUCH := 0
const AIM := 2
const PRESS := 3

## Microseconds since last zeroed: [gathering, the call, the core's own work, writing
## back]. The bench reads them; no outcome does. The write-back is the core's now (spec 35
## step 4c), so its part stays 0.
static var spent := PackedInt64Array([0, 0, 0, 0])


## One call's per-squad arrays and the bodies lying.
class Batch:
	var squad_floats := PackedFloat64Array()
	var squad_anchors := PackedVector2Array()
	var lying_ats := PackedVector2Array()
	var lying_sizes := PackedFloat64Array()


## Walks every unit of the tick's slot search (ScrumSeekField) on ctx["field"].
static func walk(ctx: Dictionary) -> void:
	var field: Object = ctx["field"]
	var seek = ctx["seek_batch"]  # ScrumSeekField.Batch, with its plan's answer
	var began := Time.get_ticks_usec()
	var batch := _gather(ctx)
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
	var bits := PackedInt32Array(
		[EntryKeys.BITS[UnitColumns.TOWARD], EntryKeys.VALUED[UnitColumns.TOWARD]]
	)
	var called := Time.get_ticks_usec()
	field.walk(
		ctx["seed"],
		floats,
		seek.squads,
		batch.squad_floats,
		batch.squad_anchors,
		seek.members,
		seek.plan[0],
		seek.plan[3],
		seek.speeds,
		bits,
		batch.lying_ats,
		batch.lying_sizes
	)
	var returned := Time.get_ticks_usec()
	spent[0] += called - began
	spent[1] += returned - called
	spent[2] += field.usec()


## Each squad's pace and frame; the bodies lying on the ground.
static func _gather(ctx: Dictionary) -> Batch:
	var batch := Batch.new()
	var crowding := BattleTuning.current().scrum_crowding
	var by_id := {}
	for squad in ctx["squads"]:
		by_id[squad.id] = squad
	var squads: PackedInt64Array = ctx["seek_batch"].squads
	for triple in range(0, squads.size(), 3):  # [id, foes, units] (ScrumSeekField)
		var squad: SkirmishSquad = by_id[squads[triple]]
		_frame(batch, squad, ctx["pace"] * (crowding if squad.pursuit.is_empty() else 1.0))
	batch.lying_ats = PackedVector2Array(ctx["lying"]["ats"])
	for body in ctx["lying"]["lying"]:
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
