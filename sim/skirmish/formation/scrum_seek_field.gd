class_name ScrumSeekField
extends RefCounted
## ScrumSeek.plan on the native core (NativeKernels, Decision 129): the field brought to the
## scrum's snapshot (BodyFieldSync), then every fighting squad's units planned in one call
## on the field's unit store (spec 35 step 4b). GDScript sends only what its rules say of
## each fighting squad - its foes, whether its units may seek, its frame, its chasers - and
## each unit's speed; the field reads each unit's place, goal and record where they live and
## writes the outcome into its loose entry's columns there, exactly as ScrumSeek's own
## seeking would: who touches a foe and stands, who keeps to its place, who makes for a
## slot (and claims it), who presses in behind a taken one. The ground is asked back only
## where there is terrain. Slots, keys, contest order and draws are the core's, bit for bit
## the GDScript's (native/README.md).

## What the core says each unit does (BodyField.seek).
enum Outcome { TOUCH, STAND, AIM, PRESS, IDLE }

const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

## Microseconds since last zeroed: [syncing the field, gathering the rules' say, the call,
## the core's own work, rebuilding the batch for the walk and the facing]. The bench reads
## them; no outcome does.
static var spent := PackedInt64Array([0, 0, 0, 0, 0])


## The units one seek planned, for the walk and the facing (ScrumWalkField, ScrumFaceField).
class Batch:
	var units: Array = []
	var owners: Array = []
	var squads := PackedInt64Array()  # per planned squad [id, foes, units]
	var foe_ids := PackedInt64Array()
	var members := PackedInt32Array()  # each unit's flat index in the sync
	var speeds := PackedFloat64Array()
	## The core's answer: [codes, unused, unused, nexts, foe_ats] (the walk's input).
	var plan: Array = []


## What the rules say of each fighting squad, in the squads' order, and each synced unit's
## speed (BodyField.seek's arguments).
class Say:
	var ids := PackedInt64Array()
	var foe_counts := PackedInt32Array()
	var foe_ids := PackedInt64Array()
	var foes := {}  # squad id -> its foes' ids
	var seeking := PackedByteArray()
	var frames := PackedFloat64Array()
	var anchors := PackedVector2Array()
	var chaser_counts := PackedInt32Array()
	var chaser_ids := PackedInt64Array()
	var speeds := PackedFloat64Array()


## Plans every fighting squad's units on ctx["field"] (see ScrumSeek.plan).
static func plan(ctx: Dictionary) -> void:
	assert(StoreHolds.none_held(ctx["squads"]), StoreHolds.SPANS)
	var field: Object = ctx["field"]
	var began := Time.get_ticks_usec()
	var synced := BodyFieldSync.scrum(field, ctx["squads"], ctx["seed"])
	var synced_at := Time.get_ticks_usec()
	spent[0] += synced_at - began
	var say := _say(synced[0], ctx["squads"])
	var called := Time.get_ticks_usec()
	var tuning := BattleTuning.current()
	var ints := PackedInt64Array(
		[
			ctx["tick"],
			ctx["seed"],
			tuning.combat_contest_die,
			EntryKeys.BITS[UnitColumns.TOUCH],
			EntryKeys.BITS[UnitColumns.FOE_AT],
		]
	)
	var floats := PackedFloat64Array([tuning.reach_contact, tuning.scrum_leash])
	var within := Callable()
	if ctx["terrain"] != null:
		var terrain: Object = ctx["terrain"]
		var units: Array = synced[0]
		within = func(flat: int, point: Vector2) -> bool:
			return terrain.factor(units[flat], point, point) > 0.0
	var out: Array = field.seek(
		ints,
		floats,
		say.ids,
		say.foe_counts,
		say.foe_ids,
		say.seeking,
		say.frames,
		say.anchors,
		say.chaser_counts,
		say.chaser_ids,
		say.speeds,
		within
	)
	var returned := Time.get_ticks_usec()
	ctx["seek_batch"] = _batch(synced, say, out)  # what the walk and the facing step
	var fighters: PackedInt64Array = out[0]
	var active: PackedByteArray = out[6]
	for index in range(fighters.size()):
		if active[index] == 1:
			ctx["active"][fighters[index]] = true
	spent[1] += called - synced_at
	spent[2] += returned - called
	spent[3] += field.usec()
	spent[4] += Time.get_ticks_usec() - returned


## What the rules say of each fighting squad - its foes, whether its units may seek (it
## isn't ordered to retreat), its frame (ScrumStance.frame), its chasers (ScrumPursuit) -
## worked out once a squad; and each synced unit's speed.
static func _say(units: Array, squads: Array) -> Say:
	var say := Say.new()
	for squad: SkirmishSquad in squads:
		if squad.state != SkirmishSquad.State.FIGHTING:
			continue
		var foes := ScrumSeek.foe_ids(squad, squads)
		say.ids.append(squad.id)
		say.foe_counts.append(foes.size())
		say.foe_ids.append_array(foes)
		say.foes[squad.id] = foes
		say.seeking.append(1 if squad.order != SkirmishUnit.Order.RETREAT else 0)
		if squad.stance.is_empty():
			say.frames.append_array([squad.heading, squad.width, squad.centre_shift])
			say.anchors.append(squad.position)
		else:
			say.frames.append_array([squad.stance["heading"], squad.width, 0.0])
			say.anchors.append(squad.stance["anchor"])
		say.chaser_counts.append(squad.chasers.size())
		say.chaser_ids.append_array(squad.chasers.keys())
	if not units.is_empty():  # the synced units share the squads' store: its SPEED, read once
		var paces: PackedFloat64Array = squads[0].store.columns.column(UnitColumns.SPEED)
		for unit: SkirmishUnit in units:
			say.speeds.append(paces[unit.slot])
	return say


## The units the seek planned, from its answer `out` (BodyField.seek), by the sync's flat
## indices (`synced` = [units, their squads]).
static func _batch(synced: Array, say: Say, out: Array) -> Batch:
	var batch := Batch.new()
	var fighters: PackedInt64Array = out[0]
	var counts: PackedInt32Array = out[2]
	for index in range(fighters.size()):
		var foes: Array = say.foes[fighters[index]]
		batch.squads.append_array([fighters[index], foes.size(), counts[index]])
		batch.foe_ids.append_array(foes)
	batch.members = out[1]
	for flat in batch.members:
		batch.units.append(synced[0][flat])
		batch.owners.append(synced[1][flat])
		batch.speeds.append(say.speeds[flat])
	batch.plan = [out[3], PackedInt64Array(), PackedInt32Array(), out[4], out[5]]
	return batch


## The units a seek on the field would plan, as GDScript reckons them, without planning:
## each fighting squad's synced units in list order, chasers left out. For the facing's
## and the walk's tests and benches, which run them on a field without a seek first.
static func gathered(units: Array, owners: Array, squads: Array) -> Batch:
	var batch := Batch.new()
	var current: SkirmishSquad = null
	var counted := 0  # the batch's units when the current squad opened
	var paces := {}  # store -> a read view of SPEED (UnitView.read_once)
	var speeds := PackedFloat64Array()  # the current squad's store's
	for index in range(units.size()):
		var squad: SkirmishSquad = owners[index]
		if squad.state != SkirmishSquad.State.FIGHTING:
			continue
		if squad != current:
			current = squad
			var foes := ScrumSeek.foe_ids(squad, squads)
			batch.squads.append_array([squad.id, foes.size(), 0])
			batch.foe_ids.append_array(foes)
			counted = batch.units.size()
			speeds = UnitView.read_once(paces, squad, [UnitColumns.SPEED]).speed
		var unit: SkirmishUnit = units[index]
		if squad.chasers.has(unit.id):
			continue
		batch.units.append(unit)
		batch.owners.append(squad)
		batch.members.append(index)
		batch.speeds.append(speeds[unit.slot])
		batch.squads[batch.squads.size() - 1] = batch.units.size() - counted
	return batch
