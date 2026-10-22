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
## still move bearing, may pick it up: the nearest pairs first, ties by the body's then the
## bearer's seeded draws - never the lists (Decision 97) - each body and bearer once.
static func _pick_up(
	squads: Array, bodies: Dictionary, tick: int, fight_seed: int, events: Array
) -> Array:
	var pairs := _pairs(squads, bodies, fight_seed)
	pairs.sort_custom(func(a, b): return ScrumContest.before(a[0], b[0]))
	var strays := []
	for pair in pairs:
		var body: SkirmishUnit = pair[1]
		var bearer: SkirmishUnit = pair[2]
		if body.state != SkirmishUnit.State.DOWNED or bearer.carrying != 0:
			continue
		UnitState.put(bodies[body.id][1], body, SkirmishUnit.State.CARRIED)
		body.carried_by = bearer.id
		bearer.carrying = body.id
		_bear(bearer, body)
		var extra := {"body": body.id}
		events.append(FormationEvents.unit_event("borne", tick, pair[3], bearer, extra))
		if pair[3].tends == "recover":
			strays.append([bearer, pair[3]])
	return strays


## [key, body, bearer, its squad] for each friendly body a free unit of a formation that
## tends its downed could pick up, as all stand (`bodies`: body id -> [body, its squad]).
## The downed are found once, and whether a foe guards each; each formation's bearers
## through a grid of where they stand.
static func _pairs(squads: Array, bodies: Dictionary, fight_seed: int) -> Array:
	var reach := BattleTuning.current().wounds_reach
	var pairs := []
	var free := {}  # faction -> [downed body no foe is near, ...]: found if any tends
	if squads.any(func(squad): return _tending(squad)):
		free = _unguarded(squads, bodies)
	var views := {}  # store -> where its units stand, read once a pass (Decision 150)
	for squad in squads:
		if not _tending(squad) or not free.has(squad.faction_id):
			continue
		var living: Array = squad.living()
		var at: PackedVector2Array = (
			UnitView.read_once(views, squad, [UnitColumns.POSITION]).position
		)
		var grid := BodyGrid.build(living.map(func(unit): return at[unit.slot]))
		for body in free[squad.faction_id]:
			for found in BodyGrid.near(grid, at[body.slot], reach + BodyGrid.MARGIN):
				var unit: SkirmishUnit = living[found]
				var gap: float = at[unit.slot].distance_to(at[body.slot])
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
## the order of `bodies`]}: the standing found through a grid of where they stand.
static func _unguarded(squads: Array, bodies: Dictionary) -> Dictionary:
	var guard := BattleTuning.current().wounds_guard_reach
	var downed := []
	for body_id in bodies:
		if bodies[body_id][0].state == SkirmishUnit.State.DOWNED:
			downed.append(bodies[body_id][0])
	if downed.is_empty():
		return {}
	var standing := []
	for squad in squads:
		standing.append_array(squad.living())
	var at: PackedVector2Array = bodies[downed[0].id][1].store.columns.column(UnitColumns.POSITION)
	var grid := BodyGrid.build(standing.map(func(unit): return at[unit.slot]))
	var free := {}
	for body in downed:
		var guarded := false
		for found in BodyGrid.near(grid, at[body.slot], guard + BodyGrid.MARGIN):
			var other: SkirmishUnit = standing[found]
			if other.faction_id != body.faction_id:
				guarded = at[other.slot].distance_to(at[body.slot]) <= guard
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
