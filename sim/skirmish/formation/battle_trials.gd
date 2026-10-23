class_name BattleTrials
extends RefCounted
## Monte Carlo trials (Decision 93): a scenario fought once per battle seed, so a rule or
## a tuning value is judged by the spread of outcomes, not by one run.
## - **mirror_headon:** 8 grems against 8 grems, head-on along a lane.
## - **mirror_flank:** 8 grems onto the side of a line of 8 grems holding its ground.
## - **field_a / field_b / field_b_waits / field_together:** the feel test's field, with
##   its content units, sent as in the scene.
## Options: "rolls" (blows rolled against their targets, Decision 118; default on),
## "captain" (the kingdom's line is led), "first_seed", "swap" (a mirror's sides spawn the
## other way round: its results must agree with the usual order within noise, Decision 97).
## A run's result: {"lost": {faction: units}, "winner": faction or "", "ticks"}; the
## player wins a field run by breaking the line.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const SCENARIOS := [
	"mirror_headon", "mirror_flank", "field_a", "field_b", "field_b_waits", "field_together"
]
const TICK := 0.1
const LIMIT_TICKS := 2000
const FACTIONS := ["player", "the_kingdom"]


## Every run's result, one per seed from options' first_seed.
static func run(scenario: String, runs: int, options: Dictionary = {}) -> Array:
	var out := []
	for index in range(runs):
		var battle_seed: int = options.get("first_seed", 1) + index
		if scenario.begins_with("mirror"):
			out.append(_mirror(scenario, battle_seed, options))
		else:
			out.append(_field(scenario, battle_seed, options))
	return out


## {"runs", "wins": {faction: n}, "lost": {faction: [mean, sd, min, max]}, "ticks": mean}.
static func summary(results: Array) -> Dictionary:
	var wins := {"player": 0, "the_kingdom": 0, "": 0}
	var lost := {}
	var ticks := 0.0
	for faction in FACTIONS:
		var values: Array = results.map(func(r): return float(r["lost"].get(faction, 0)))
		lost[faction] = _spread(values)
	for result in results:
		wins[result["winner"]] += 1
		ticks += result["ticks"]
	return {
		"runs": results.size(), "wins": wins, "lost": lost, "ticks": ticks / maxi(1, results.size())
	}


static func _spread(values: Array) -> Array:
	if values.is_empty():
		return [0.0, 0.0, 0.0, 0.0]
	var mean: float = values.reduce(func(a, v): return a + v, 0.0) / values.size()
	var square: float = values.reduce(func(a, v): return a + (v - mean) * (v - mean), 0.0)
	return [mean, sqrt(square / values.size()), values.min(), values.max()]


static func _grem() -> UnitDef:
	return CrateContent.unit("grem")


static func _row(unit_def: UnitDef, columns: int) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


## The scenario's spawns, in its usual order, as callables.
static func _mirror_spawns(scenario: String, sim: FormationSimulation) -> Array:
	var grem := _grem()
	if scenario == "mirror_headon":
		return [
			func(): sim.spawn_squad(8, _row(grem, 8), "player", true),
			func(): sim.spawn_squad(8, _row(grem, 8), "the_kingdom", false),
		]
	var hold := FormationRoute.new(PackedVector2Array([Vector2(64, 32), Vector2(0, 32)]))
	var down := FormationRoute.new(PackedVector2Array([Vector2(64.5, 0), Vector2(64.5, 64)]))
	return [
		func():
			var line := sim.spawn_squad(8, _row(grem, 8), "the_kingdom", true, 0, hold)
			sim.order(line.id, SkirmishUnit.Order.HOLD),
		func(): sim.spawn_squad(8, _row(grem, 8), "player", true, 0, down),
	]


static func _mirror(scenario: String, battle_seed: int, options: Dictionary) -> Dictionary:
	var sim := FormationSimulation.new(2.0, TICK)
	sim.fight_seed = battle_seed
	sim.blow_rolls = options.get("rolls", true)
	var spawns := _mirror_spawns(scenario, sim)
	if options.get("swap", false):
		spawns.reverse()  # the other side first: outcomes must not move (Decision 97)
	for spawn in spawns:
		spawn.call()
	var lost := {"player": 0, "the_kingdom": 0}
	var seen := {}
	for tick in range(LIMIT_TICKS):
		_count(sim.step(), lost, seen)
		var standing := FACTIONS.filter(func(f): return _stands(sim.squads(), f))
		if standing.size() < 2:
			var winner: String = standing[0] if standing.size() == 1 else ""
			return {"lost": lost, "winner": winner, "ticks": tick + 1}
	return {"lost": lost, "winner": "", "ticks": LIMIT_TICKS}


## True if the faction has a squad still standing in the fight (not destroyed or routing).
static func _stands(squads: Array, faction: String) -> bool:
	return squads.any(
		func(s):
			return (
				s.faction_id == faction
				and not s.is_destroyed()
				and s.state != SkirmishSquad.State.ROUTING
			)
	)


static func _field(scenario: String, battle_seed: int, options: Dictionary) -> Dictionary:
	var captain: UnitDef = (
		CrateContent.unit("kingdom_captain") if options.get("captain", false) else null
	)
	var field := FormationField.new(
		TICK,
		_grem(),
		8,
		CrateContent.unit("kingdom_militia"),
		CrateContent.unit("grem_chieftain"),
		captain,
		battle_seed
	)
	field.sim.blow_rolls = options.get("rolls", true)
	for _i in range(LIMIT_TICKS):
		if field.waves["A"].built() == 8 and field.waves["B"].built() == 9:  # B's chieftain
			break
		field.step()
	var lost := {"player": 0, "the_kingdom": 0}
	var events := _send(field, scenario)
	for tick in range(LIMIT_TICKS):
		events.append_array(field.step())
		var line := field.kingdom_line
		if line.is_destroyed() or line.state == SkirmishSquad.State.ROUTING:
			_count(events, lost, {})
			return {"lost": lost, "winner": "player", "ticks": tick + 1}
	_count(events, lost, {})
	return {"lost": lost, "winner": "the_kingdom", "ticks": LIMIT_TICKS}


static func _send(field: FormationField, scenario: String) -> Array:
	var events := []
	match scenario:
		"field_a":
			field.send("A")
		"field_b":
			field.send("B")
		"field_b_waits":
			field.set_wait(true)
			field.send("B")
			for _i in range(90):
				events.append_array(field.step())
			field.send("A")
		_:
			field.send_together(["A", "B"])
	return events


## A unit lost left the fight - killed outright, downed, or surrendered (Decision 121) -
## counted once, though it may come to and fall again (Decision 126). `seen`: unit ids.
static func _count(events: Array, lost: Dictionary, seen: Dictionary) -> void:
	for event in events:
		if event["type"] in ["died", "downed", "surrendered"] and not seen.has(event["unit"]):
			seen[event["unit"]] = true
			lost[event["faction"]] += 1
