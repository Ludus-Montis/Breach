class_name FormationCarry
extends RefCounted
## Bearing the wounded (Decision 126). A formation told to tend its downed - "recover" or
## "carry" - has a free unit beside each friendly downed body pick it up, while it isn't
## fighting and no foe is near the body: the nearest free unit that can still move under
## it. The body is load (wounds_body_weight a cell of it), slowing and tiring its bearer.
## "recover": the bearer sets off home with it alone (FormationStrays), and when it gets
## there both return to the reserve; "carry": the bearer keeps its place, bearing it on.
## A bearer that falls drops its body where it stands; a body that comes to is set down.
## A borne body still comes to, or dies of its wounds (FormationRecovery). Pure over the
## squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")


## One tick: bodies follow their bearers, are dropped or set down, come home with them,
## and are picked up. `events` are this tick's so far (a bearer's "fled_home"). Returns
## [[bearer, its squad], ...] setting off home alone with a body.
static func step(squads: Array, tick: int, fight_seed: int, events: Array) -> Array:
	var units := {}  # unit id -> [unit, its squad]
	for squad in squads:
		for unit in squad.units:
			units[unit.id] = [unit, squad]
	_bring_home(units, tick, events)
	for unit_id in units:
		_follow(units[unit_id][0], units)
	return _pick_up(squads, units, tick, fight_seed, events)


## The weight its body is, borne.
static func body_weight(body: SkirmishUnit) -> float:
	return body.footprint_width * body.footprint_depth * BattleTuning.current().wounds_body_weight


## Bodies whose bearers fled home this tick come home too.
static func _bring_home(units: Dictionary, tick: int, events: Array) -> void:
	var homes := events.filter(func(e): return e["type"] == "fled_home")
	for event in homes:
		for unit_id in units:
			var body: SkirmishUnit = units[unit_id][0]
			if body.state == SkirmishUnit.State.CARRIED and body.carried_by == event["unit"]:
				units[unit_id][1].units.erase(body)
				var extra := {"unit": body.id, "definition": body.definition}
				events.append(
					FormationEvents.squad_event("fled_home", tick, units[unit_id][1], extra)
				)


## A body follows its bearer; a bearer down drops it, and one come to is set down.
static func _follow(unit: SkirmishUnit, units: Dictionary) -> void:
	if unit.state == SkirmishUnit.State.CARRIED:
		var bearer: SkirmishUnit = units[unit.carried_by][0] if units.has(unit.carried_by) else null
		if bearer == null or not bearer.is_alive():
			unit.state = SkirmishUnit.State.DOWNED  # dropped where it is
			unit.carried_by = 0
		else:
			# placement: a carried body is where its bearer is (Decision 139)
			unit.position = bearer.position
	if unit.carrying != 0:
		var body: SkirmishUnit = units[unit.carrying][0] if units.has(unit.carrying) else null
		if body == null or body.state != SkirmishUnit.State.CARRIED or not unit.is_alive():
			unit.carrying = 0
			_bear(unit, null)


## Every free unit of a formation that tends its downed, beside a friendly body it can
## still move bearing, may pick it up: the nearest pairs first, ties by the body's then the
## bearer's seeded draws - never the lists (Decision 97) - each body and bearer once.
static func _pick_up(
	squads: Array, units: Dictionary, tick: int, fight_seed: int, events: Array
) -> Array:
	var pairs := _pairs(squads, units, fight_seed)
	pairs.sort_custom(func(a, b): return ScrumContest.before(a[0], b[0]))
	var strays := []
	for pair in pairs:
		var body: SkirmishUnit = pair[1]
		var bearer: SkirmishUnit = pair[2]
		if body.state != SkirmishUnit.State.DOWNED or bearer.carrying != 0:
			continue
		body.state = SkirmishUnit.State.CARRIED
		body.carried_by = bearer.id
		bearer.carrying = body.id
		_bear(bearer, body)
		var extra := {"body": body.id}
		events.append(FormationEvents.unit_event("borne", tick, pair[3], bearer, extra))
		if pair[3].tends == "recover":
			strays.append([bearer, pair[3]])
	return strays


## [key, body, bearer, its squad] for each friendly body a free unit of a formation that
## tends its downed could pick up, as all stand. The downed are found once, and whether a
## foe guards each; each formation's bearers through a grid of where they stand.
static func _pairs(squads: Array, units: Dictionary, fight_seed: int) -> Array:
	var reach := BattleTuning.current().wounds_reach
	var pairs := []
	var free := {}  # faction -> [downed body no foe is near, ...]: found if any tends
	if squads.any(func(squad): return _tending(squad)):
		free = _unguarded(units)
	for squad in squads:
		if not _tending(squad) or not free.has(squad.faction_id):
			continue
		var living: Array = squad.living()
		var grid := BodyGrid.build(living.map(func(unit): return unit.position))
		for body in free[squad.faction_id]:
			for found in BodyGrid.near(grid, body.position, reach + BodyGrid.MARGIN):
				var unit: SkirmishUnit = living[found]
				var gap: float = unit.position.distance_to(body.position)
				if unit.carrying != 0 or gap > reach or _stage(unit, body) >= 3:
					continue
				var key := [snappedf(gap, 0.000001), ScrumContest.draw(body, fight_seed)]
				key.append(ScrumContest.draw(unit, fight_seed))
				pairs.append([key, body, unit, squad])
	return pairs


## True if the squad tends its downed now: told to, and not fighting or routing.
static func _tending(squad: SkirmishSquad) -> bool:
	if squad.tends == "":
		return false
	return (
		squad.state != SkirmishSquad.State.FIGHTING and squad.state != SkirmishSquad.State.ROUTING
	)


## {faction: [each downed body of it with no foe standing within wounds_guard_reach, in
## the order of `units`]}: the standing found through a grid of where they stand.
static func _unguarded(units: Dictionary) -> Dictionary:
	var guard := BattleTuning.current().wounds_guard_reach
	var standing := []
	var downed := []
	for unit_id in units:
		var unit: SkirmishUnit = units[unit_id][0]
		if unit.is_alive():
			standing.append(unit)
		elif unit.state == SkirmishUnit.State.DOWNED:
			downed.append(unit)
	if downed.is_empty():
		return {}
	var grid := BodyGrid.build(standing.map(func(unit): return unit.position))
	var free := {}
	for body in downed:
		var guarded := false
		for found in BodyGrid.near(grid, body.position, guard + BodyGrid.MARGIN):
			var other: SkirmishUnit = standing[found]
			if other.faction_id != body.faction_id:
				guarded = other.position.distance_to(body.position) <= guard
				if guarded:
					break
		if not guarded:
			if not free.has(body.faction_id):
				free[body.faction_id] = []
			free[body.faction_id].append(body)
	return free


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
