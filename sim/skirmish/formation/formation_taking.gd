class_name FormationTaking
extends RefCounted
## Taking the downed (spec 30 round 3, part 8; Decisions 121 and 124: "units must either
## kill or capture"). A group out of a fight sends its free units - in their places, not
## loose or fleeing - out to the downed foes it can see within take_reach of it, one unit to
## a body, nearest first; each walks to its body (FormationWalk.target_of) and, beside it,
## takes it by the wounds rules (FormationWounds: finish, capture, messenger, playing dead).
## The group holds while its takers are out, up to take_hold seconds - for good with "no
## man left behind" - and under "fall behind, left behind", or when its command leaves the
## downed, it presses on and takes none. A taker comes back to its place once its body is
## dead, taken, borne away or come to, or the hold runs out. Pairs go by distance, then the
## units' and bodies' seeded draws, never by ids or lists (Decision 97). A group that is
## threatened - it sees a standing foe (FormationPerception, Decision 134) - fights first:
## it sends no takers, calls back those out and doesn't hold for them. Who goes out to
## which body is the crate's (`taking.rs`, spec 39 step 6c). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationPerception = preload("res://sim/skirmish/formation/formation_perception.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const AT_EASE := [
	SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING, SkirmishSquad.State.ARRIVED
]


## One tick: takers whose work is done come back, then groups send free units out - none
## from a group threatened in `perception` (FormationPerception.see), whose takers all
## come back.
static func step(
	squads: Array, tick: int, seconds: float, fight_seed: int, events: Array, perception := {}
):
	var downed := []
	for squad in squads:
		for unit in UnitState.downed(squad):
			if unit.state == SkirmishUnit.State.DOWNED:  # a borne body isn't taken
				downed.append(unit)
	var sending := []
	for squad in squads:
		var threatened := FormationPerception.threatened(perception, squad)
		_recall(squad, tick, seconds, threatened)
		# one that gave up on these marches on
		if _takes(squad) and not threatened and squad.took_until < 0:
			sending.append(squad)
	if not sending.is_empty() and not downed.is_empty():
		_send(sending, downed, tick, fight_seed, events)


## True while the squad holds for its takers.
static func holding(squad: SkirmishSquad) -> bool:
	return not squad.taking.is_empty()


static func _takes(squad: SkirmishSquad) -> bool:
	return (
		squad.state in AT_EASE
		and squad.command.takes
		and FormationWalk.rule_of(squad) != "fall_behind_left_behind"
	)


## Takers come back once their bodies are no longer downed, or the hold has run out.
static func _recall(squad: SkirmishSquad, tick: int, seconds: float, threatened: bool) -> void:
	if squad.state == SkirmishSquad.State.FIGHTING:
		squad.took_until = -1  # a new fight: its downed foes are worth taking again
	if squad.taking.is_empty():
		return
	var held := (tick - squad.taking_since) * seconds
	var lasting := FormationWalk.rule_of(squad) == "no_man_left_behind"
	var out_of_time: bool = not lasting and held >= BattleTuning.current().take_hold
	var at_ease: bool = squad.state in AT_EASE  # read once (Decision 144)
	for unit_id in squad.taking.keys():
		var body: SkirmishUnit = squad.taking[unit_id]
		var taker = squad.units.filter(func(u): return u.id == unit_id)
		if out_of_time or body.state != SkirmishUnit.State.DOWNED or taker.is_empty():
			squad.taking.erase(unit_id)
		elif threatened or not taker[0].is_alive() or not at_ease:
			squad.taking.erase(unit_id)
	if squad.taking.is_empty():
		squad.taking_since = -1
		squad.took_until = tick if out_of_time else -1


## Pairs each sending squad's free units with downed foes they can see within take_reach:
## the crate decides (`taking.rs`: nearest first, ties by the seeded draws, each once), and
## this lands it - the takings, the hold's start and the events.
static func _send(sending: Array, downed: Array, tick: int, fight_seed: int, events: Array) -> void:
	var codes := {}  # faction -> its code
	var index := {}  # body -> its index in `downed`
	var lying := PackedInt64Array()
	for body: SkirmishUnit in downed:
		index[body] = index.size()
		lying.append_array([body.id, body.slot, codes.get_or_add(body.faction_id, codes.size())])
	var counts := PackedInt32Array()
	var takers := PackedInt64Array()
	var detections := PackedFloat64Array()
	var claimed := PackedInt32Array()
	var listed := []  # per squad, its free units as handed in
	for squad: SkirmishSquad in sending:
		var free := squad.living().filter(
			func(unit):
				return not (
					squad.loose.has(unit.id)
					or squad.fleeing.has(unit.id)
					or squad.taking.has(unit.id)
				)
		)
		listed.append(free)
		var claims := squad.taking.values().filter(func(body): return index.has(body))
		counts.append_array([free.size(), claims.size()])
		for unit: SkirmishUnit in free:
			var code: int = codes.get_or_add(unit.faction_id, codes.size())
			takers.append_array([unit.id, unit.slot, code])
			detections.append(unit.detection)
		for body in claims:
			claimed.append(index[body])
	var columns: UnitColumns = sending[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var reach := BattleTuning.current().take_reach
	var sent: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return field.taking_send(counts, takers, detections, claimed, lying, fight_seed, reach)
	)
	for at in range(0, sent.size(), 3):
		var squad: SkirmishSquad = sending[sent[at]]
		var unit: SkirmishUnit = listed[sent[at]][sent[at + 1]]
		squad.taking[unit.id] = downed[sent[at + 2]]
		if squad.taking_since < 0:
			squad.taking_since = tick
		events.append(FormationEvents.unit_event("sent_to_take", tick, squad, unit))
