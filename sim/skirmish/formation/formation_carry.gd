class_name FormationCarry
extends RefCounted
## Bearing the wounded (Decision 126). A formation told to tend its downed - "recover" or
## "carry" - has a free unit beside each friendly downed body pick it up, while it isn't
## fighting and no foe is near the body: the nearest free unit that can still move under
## it. The body is load (wounds_body_weight a cell of it), slowing and tiring its bearer.
## "recover": the bearer sets off home with it alone (FormationStrays), and when it gets
## there both return to the reserve; "carry": the bearer keeps its place, bearing it on.
## A bearer that falls drops its body where it stands; a body that comes to is set down.
## A borne body still comes to, or dies of its wounds (FormationRecovery). Who picks up whom
## is the crate's (`carry.rs`, spec 39 step 6c). Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## One tick: bodies follow their bearers, are dropped or set down, come home with them,
## and are picked up. `events` are this tick's so far (a bearer's "fled_home"). Returns
## [[bearer, its squad], ...] setting off home alone with a body.
static func step(squads: Array, tick: int, fight_seed: int, events: Array) -> Array:
	var bodies := {}  # body id -> [body, its squad]: the downed, in the squads' order
	var bearers := {}  # bearer id -> [bearer, its squad], standing or not
	for squad in squads:
		for body in UnitState.downed(squad):
			bodies[body.id] = [body, squad]
		for unit in squad.units:
			if unit.carrying != 0:
				bearers[unit.id] = [unit, squad]
	_bring_home(bodies, tick, events)
	for body_id in bodies:
		_follow(bodies[body_id], bearers)
	for bearer_id in bearers:
		_hold(bearers[bearer_id][0], bodies)
	return _pick_up(squads, bodies, tick, fight_seed, events)


## The weight its body is, borne.
static func body_weight(body: SkirmishUnit) -> float:
	return body.footprint_width * body.footprint_depth * BattleTuning.current().wounds_body_weight


## Bodies whose bearers fled home this tick come home too.
static func _bring_home(bodies: Dictionary, tick: int, events: Array) -> void:
	var homes := events.filter(func(e): return e["type"] == "fled_home")
	for event in homes:
		for body_id in bodies:
			var body: SkirmishUnit = bodies[body_id][0]
			var squad: SkirmishSquad = bodies[body_id][1]
			if body.state == SkirmishUnit.State.CARRIED and body.carried_by == event["unit"]:
				squad.units.erase(body)
				UnitState.regather(squad)
				var extra := {"unit": body.id, "definition": body.definition}
				events.append(FormationEvents.squad_event("fled_home", tick, squad, extra))


## A body ([body, its squad]) follows its bearer; one whose bearer is down or gone is
## dropped where it is.
static func _follow(entry: Array, bearers: Dictionary) -> void:
	var body: SkirmishUnit = entry[0]
	if body.state != SkirmishUnit.State.CARRIED:
		return
	var bearer: SkirmishUnit = bearers[body.carried_by][0] if bearers.has(body.carried_by) else null
	if bearer == null or not bearer.is_alive():
		UnitState.put(entry[1], body, SkirmishUnit.State.DOWNED)  # dropped where it is
		body.carried_by = 0
	else:
		var columns: UnitColumns = entry[1].store.columns
		# placement: a carried body is where its bearer is (Decision 139)
		columns.put(
			body.slot, UnitColumns.POSITION, columns.value(bearer.slot, UnitColumns.POSITION)
		)


## A bearer that is down, or whose body is no longer borne (it came to, died or went
## home), sets its load down.
static func _hold(unit: SkirmishUnit, bodies: Dictionary) -> void:
	var body: SkirmishUnit = bodies[unit.carrying][0] if bodies.has(unit.carrying) else null
	if body == null or body.state != SkirmishUnit.State.CARRIED or not unit.is_alive():
		unit.carrying = 0
		_bear(unit, null)


## Every free unit of a formation that tends its downed, beside a friendly body it can
## still move bearing, may pick it up: the crate decides who bears whom (`carry.rs`: the
## nearest pairs first, ties by the seeded draws, each body and bearer once), and this lands
## it - the states, the load, the events and those setting off home.
static func _pick_up(
	squads: Array, bodies: Dictionary, tick: int, fight_seed: int, events: Array
) -> Array:
	if not squads.any(func(squad): return _tending(squad)):
		return []
	var listed := []  # per squad, [its living, its downed bodies lying], as handed in
	var counts := PackedInt32Array()
	var ints := PackedInt64Array()
	var gear := PackedFloat64Array()
	var codes := {}  # faction -> its code
	for squad in squads:
		var living: Array = squad.living()
		var lying := UnitState.downed(squad).filter(
			func(unit): return unit.state == SkirmishUnit.State.DOWNED
		)
		listed.append([living, lying])
		var code: int = codes.get_or_add(squad.faction_id, codes.size())
		counts.append_array([code, int(_tending(squad)), living.size(), lying.size()])
		for unit: SkirmishUnit in living:
			var strength := int(unit.attributes.get("strength", 10))
			var hauler := int(unit.traits.get("hauler", 0))
			var faction: int = codes.get_or_add(unit.faction_id, codes.size())
			ints.append_array(
				[unit.id, unit.slot, faction, int(unit.carrying != 0), strength, hauler]
			)
			gear.append(unit.gear_weight)
		for body: SkirmishUnit in lying:
			var faction: int = codes.get_or_add(body.faction_id, codes.size())
			var cells := body.footprint_width * body.footprint_depth
			ints.append_array([body.id, body.slot, faction, cells, 0, 0])
			gear.append(0.0)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := tuning()
	var picked: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.carry_pick_up(counts, ints, gear, fight_seed, tuning)
	)
	var strays := []
	for at in range(0, picked.size(), 4):
		var body: SkirmishUnit = listed[picked[at]][1][picked[at + 1]]
		var squad: SkirmishSquad = squads[picked[at + 2]]
		var bearer: SkirmishUnit = listed[picked[at + 2]][0][picked[at + 3]]
		UnitState.put(bodies[body.id][1], body, SkirmishUnit.State.CARRIED)
		body.carried_by = bearer.id
		bearer.carrying = body.id
		_bear(bearer, body)
		var extra := {"body": body.id}
		events.append(FormationEvents.unit_event("borne", tick, squad, bearer, extra))
		if squad.tends == "recover":
			strays.append([bearer, squad])
	return strays


## What the pick-up takes of BattleTuning, in the order the crate reads it (CarryTuning).
static func tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			tuning.wounds_reach,
			tuning.wounds_guard_reach,
			tuning.wounds_body_weight,
			tuning.load_per_strength,
			tuning.load_easy,
			tuning.load_most,
			tuning.load_hauler,
		]
	)


## True if the squad tends its downed now: told to, and not fighting or routing.
static func _tending(squad: SkirmishSquad) -> bool:
	if squad.tends == "":
		return false
	return (
		squad.state != SkirmishSquad.State.FIGHTING and squad.state != SkirmishSquad.State.ROUTING
	)


## Its load stage bearing `body` (null: none).
static func _stage(unit: SkirmishUnit, body: SkirmishUnit) -> int:
	var weight := unit.gear_weight + (body_weight(body) if body != null else 0.0)
	var strength := int(unit.attributes.get("strength", 10))
	return UnitArms.stage_of(weight, strength, int(unit.traits.get("hauler", 0)))


## Sets the unit's pace, tiring and dodge for its load bearing `body` (null: none).
static func _bear(unit: SkirmishUnit, body: SkirmishUnit) -> void:
	var tuning := BattleTuning.current()
	var stage := _stage(unit, body)
	var march: float = unit.definition.speed if unit.definition != null else unit.fresh_speed
	unit.load_stage = stage
	unit.fresh_speed = march * tuning.load_pace[stage]
	unit.tiring = tuning.load_tiring[stage]
	unit.dodging = tuning.load_dodge[stage]
