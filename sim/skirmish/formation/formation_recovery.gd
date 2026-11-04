class_name FormationRecovery
extends RefCounted
## What the wounded come to (Decisions 121, 125 and 126). A unit's **condition** is 1 less
## wounds_condition for each wound past those it is hardened to (a "hardened N" trait), and
## later its other statuses; below condition_drain_below it loses HP, standing or downed,
## the faster the further below (condition_drain a second at condition 0) - so two wounds
## in good condition are survivable, a third bleeds it out unless it is aided soon. A
## **downed** unit lies a seeded while (wounds_wake_seconds, shorter the hardier it is) and
## then comes to at 1 HP where it lies: it joins the nearest standing friendly formation it
## can see - its own, or one that bore it - walking to a place at its back, and otherwise
## makes for home on its own, as a lone router (FormationStrays), to be caught, taken, or
## reach the reserve. With a foe within a few cells when it could get up, it may play dead
## instead (Decision 127). Regeneration mends here too (`regeneration.rs`). The pass is the
## crate's (`recovery/pass.rs`, spec 39 step 8b5-xxv): it reads the squads' living and downed
## where they live, decides and lands it - the deaths, those playing dead, those coming to
## joining a formation or setting off alone, and the events. This mirrors the states, the
## playing dead and the squads' lists. Pure over the squads it is given.

const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")


## One tick of `seconds`. Returns [[unit, its squad], ...] for those that came to apart from
## their formation, to make for home (FormationStrays).
static func step(squads: Array, seconds: float, tick: int, fight_seed: int, events: Array) -> Array:
	if squads.is_empty():
		return []
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var at := PackedInt64Array([tick, fight_seed])
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := FormationWounds.tuning()
	var landed: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.recovery_step(listed, at, seconds, tuning)
	)
	var strays := mirror(squads, landed)
	events.append_array(CrateEvents.taken(field))  # "died_of_wounds", "came_to", ...
	return strays


## GDScript's copies of what `recovery_step` landed (four numbers a unit: its squad's listed
## index, its id, what befell it, the squad it joined or -1). Returns the strays, [[unit, its
## squad], ...], for FormationStrays.adopt.
static func mirror(squads: Array, landed: PackedInt64Array) -> Array:
	var strays := []
	for n in range(0, landed.size(), 4):  # store: scattered (few: the downed it befell)
		var own: SkirmishSquad = squads[landed[n]]
		var unit: SkirmishUnit = own.store.record_of(landed[n + 1])
		if landed[n + 2] == 0:
			UnitState.put(own, unit, SkirmishUnit.State.DEAD)
		elif landed[n + 2] == 1:
			unit.playing_dead = true
		elif landed[n + 3] < 0:
			_rose(unit, own, null)
			strays.append([unit, own])
		else:
			_rose(unit, own, squads[landed[n + 3]])
	return strays


## The unit came to and is up, joining `best` (null: none, to make for home alone): its
## state, its move from its own squad's list to `best`'s and `best` reforming, mirrored.
static func _rose(unit: SkirmishUnit, own: SkirmishSquad, best: SkirmishSquad) -> void:
	UnitState.put(own, unit, SkirmishUnit.State.MOVING)
	unit.playing_dead = false
	if best == null:
		return
	if best != own:
		own.units.erase(unit)
		UnitState.regather(own)
		FormationCommand.enlist(best, unit)
	best.reforming = true
