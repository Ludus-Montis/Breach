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
## instead (Decision 127). Regeneration mends here too (`regeneration.rs`). The rules are the
## crate's (`recovery.rs`, spec 39 step 6b): this hands it the squads' living and downed with
## what of each it doesn't hold, and lands what it decides - the deaths, those playing dead,
## those coming to joining a formation or setting off alone, and the events. Pure over the
## squads it is given.

const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## One tick of `seconds`. Returns [[unit, its squad], ...] for those that came to apart from
## their formation, to make for home (FormationStrays).
static func step(squads: Array, seconds: float, tick: int, fight_seed: int, events: Array) -> Array:
	if squads.is_empty():
		return []
	var listed := PackedInt64Array()
	var counts := PackedInt32Array()
	var ints := PackedInt64Array()
	var reals := PackedFloat64Array()
	var downed := []  # per squad, its downed as handed in
	var codes := {}  # faction -> its code
	for squad: SkirmishSquad in squads:
		var living: Array = squad.living()
		var lying: Array = UnitState.downed(squad)
		downed.append(lying)
		var joinable := (
			squad.state not in [SkirmishSquad.State.DESTROYED, SkirmishSquad.State.ROUTING]
		)
		listed.append_array([squad.id, int(joinable)])
		counts.append_array([living.size(), lying.size()])
		for unit: SkirmishUnit in living + lying:
			(
				ints
				. append_array(
					[
						unit.id,
						unit.slot,
						codes.get_or_add(unit.faction_id, codes.size()),
						unit.wounded,
						int(unit.traits.get("hardened", 0)),
						int(unit.traits.get("cunning", 0)),
						_flags(unit),
						unit.max_hp,
					]
				)
			)
			(
				reals
				. append_array(
					[
						unit.regeneration,
						unit.attributes.get("wits", UnitDef.AVERAGE),
						unit.attributes.get("constitution", UnitDef.AVERAGE),
						unit.detection,
					]
				)
			)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := FormationWounds.tuning()
	var decided: PackedInt64Array = ColumnsLent.all(
		columns,
		func(): return field.recovery_step(listed, counts, ints, reals, seconds, fight_seed, tuning)
	)
	var end := 1 + 3 * decided[0]
	for at in range(1, end, 3):
		var squad: SkirmishSquad = squads[decided[at + 1]]
		var unit: SkirmishUnit = downed[decided[at + 1]][decided[at + 2]]
		if decided[at] == 0:
			UnitState.put(squad, unit, SkirmishUnit.State.DEAD)
			events.append(FormationEvents.unit_event("died_of_wounds", tick, squad, unit))
		else:
			unit.playing_dead = true
			events.append(FormationEvents.unit_event("playing_dead", tick, squad, unit))
	var strays := []
	for at in range(end, decided.size(), 3):  # store: scattered (few: those coming to)
		var own: SkirmishSquad = squads[decided[at]]
		var unit: SkirmishUnit = downed[decided[at]][decided[at + 1]]
		var best: SkirmishSquad = squads[decided[at + 2]] if decided[at + 2] >= 0 else null
		UnitState.put(own, unit, SkirmishUnit.State.MOVING)
		unit.playing_dead = false
		var extra := {"alone": best == null}
		if best == null:
			strays.append([unit, own])
		else:
			_join(unit, own, best)
			extra["joined"] = best.id
		events.append(FormationEvents.unit_event("came_to", tick, own, unit, extra))
	return strays


## A unit's flags as the crate takes them: 1 it leads, 2 it mends downed, 4 it plays dead.
static func _flags(unit: SkirmishUnit) -> int:
	var mends := int(unit.traits.get("regenerates_downed", 0) > 0)
	return int(unit.leadership > 0) | mends << 1 | int(unit.playing_dead) << 2


## The unit joins `best`, walking from where it lay to a place at its back.
static func _join(unit: SkirmishUnit, own: SkirmishSquad, best: SkirmishSquad) -> void:
	var back := 0
	var ranks: PackedInt32Array = UnitView.read(best, [UnitColumns.RANK]).rank  # read once
	for other in best.living():
		if other != unit:
			back = maxi(back, ranks[other.slot] + other.footprint_depth)
	if best != own:
		SquadEntries.unloose(own, unit.id)
		SquadEntries.unflee(own, unit.id)
		own.units.erase(unit)
		UnitState.regather(own)
		FormationCommand.enlist(best, unit)
	UnitPlace.put(best, unit, Vector2i(back, 0))
	best.reforming = true
	var at: Vector2 = best.store.columns.value(unit.slot, UnitColumns.POSITION)
	SquadEntries.loosen(best, unit, at)
