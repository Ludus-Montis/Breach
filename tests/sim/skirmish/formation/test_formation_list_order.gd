extends GdUnitTestSuite
## No outcome hangs on list order (Decision 97): the squads and their units are listed in
## spawn order, so a battle stepped with every list reversed - the same ids, the same seed -
## must come out the same, tick for tick; where squads tie on where they stand, the seeded
## draw decides, not the list.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(hp: int = 400) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return unit_def


func _row(columns: int, hp: int = 400) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([_def(hp), Vector2i(0, column)])
	return placements


## Everything an outcome is made of, by id: each squad's state, place and fight, and each
## unit's hp, place, bearing and target.
func _state(squads: Array) -> String:
	var rows := []
	for squad in squads:
		var units := []
		for unit in squad.units:
			var at: Vector2 = UnitPlaces.of(squad, unit).snapped(Vector2(0.0001, 0.0001))
			var facing := UnitPlaces.bearing_of(squad, unit)
			units.append(
				[unit.id, UnitHealth.of_one(squad, unit), UnitPlace.of(squad, unit), at, facing]
			)
		units.sort()
		var flanks := {}
		for edge in squad.flank_contacts:
			flanks[edge] = squad.flank_contacts[edge]["foe"]
		var front := snappedf(squad.front_distance, 0.000001)
		var fight := [squad.engaged_with, flanks, squad.morale]
		rows.append([squad.id, squad.state, front, squad.facing, fight, units])
	rows.sort()
	return str(rows)


func _reverse(squads: Array) -> void:
	squads.reverse()
	for squad in squads:
		squad.units.reverse()
		UnitState.regather(squad)


## The first tick (1-based) at which a battle and its list-reversed twin differ; 0 if none.
func _first_difference(make: Callable, step: Callable, ticks: int) -> int:
	var usual: FormationSimulation = make.call()
	var reversed: FormationSimulation = make.call()
	for tick in range(ticks):
		step.call(usual, tick)
		_reverse(reversed.squads())
		step.call(reversed, tick)
		if _state(usual.squads()) != _state(reversed.squads()):
			return tick + 1
	return 0


func _mirror(scenario: String, battle_seed: int) -> Callable:
	return func():
		var sim := FormationSimulation.new(2.0, TICK)
		sim.fight_seed = battle_seed
		sim.blow_rolls = true
		for spawn in BattleTrials._mirror_spawns(scenario, sim):
			spawn.call()
		return sim


func _plain_step(sim: FormationSimulation, _tick: int) -> void:
	sim.step()


func test_a_head_on_mirror_is_the_same_with_its_lists_reversed() -> void:
	for battle_seed in [1, 2]:
		var tick := _first_difference(_mirror("mirror_headon", battle_seed), _plain_step, 400)
		assert_int(tick).is_equal(0)


func test_a_flank_mirror_is_the_same_with_its_lists_reversed() -> void:
	for battle_seed in [1, 2]:
		var tick := _first_difference(_mirror("mirror_flank", battle_seed), _plain_step, 300)
		assert_int(tick).is_equal(0)


## A player line facing two kingdom squads side by side at the same gap.
func _between_two(battle_seed: int) -> Callable:
	return func():
		var sim := FormationSimulation.new(2.0, TICK)
		sim.fight_seed = battle_seed
		sim.spawn_squad(4, _row(4), "player", true)
		for _i in range(2):
			sim.spawn_squad(2, _row(2), "the_kingdom", false)
		return sim


## The id of the squad the player's line locked onto first.
func _locked_onto(sim: FormationSimulation) -> int:
	for _i in range(400):
		sim.step()
		var line: SkirmishSquad = sim.squads().filter(func(s): return s.faction_id == "player")[0]
		if line.engaged_with != 0:
			return line.engaged_with
	return 0


func test_a_squad_between_two_equal_foes_locks_onto_one_by_the_draw_not_the_list() -> void:
	var picked := {}
	for battle_seed in range(1, 13):
		var make := _between_two(battle_seed)
		var usual: FormationSimulation = make.call()
		var reversed: FormationSimulation = make.call()
		_reverse(reversed.squads())
		var foe := _locked_onto(usual)
		assert_int(_locked_onto(reversed)).is_equal(foe)
		picked[foe] = true
	assert_int(picked.size()).is_equal(2)  # each foe is the draw's pick in some battles


func test_a_squad_between_two_equal_foes_is_the_same_with_its_lists_reversed() -> void:
	assert_int(_first_difference(_between_two(3), _plain_step, 300)).is_equal(0)


## [player front x, kingdom front x] when lines that march at each other engage, the
## kingdom spawned first if asked.
func _meeting(kingdom_first: bool) -> Array:
	var sim := FormationSimulation.new(2.0, TICK)
	var spawns := [
		func(): return sim.spawn_squad(8, _row(8), "player", true),
		func(): return sim.spawn_squad(8, _row(8), "the_kingdom", false),
	]
	if kingdom_first:
		spawns.reverse()
	var squads := spawns.map(func(spawn): return spawn.call())
	if kingdom_first:
		squads.reverse()
	for _i in range(300):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	return [squads[0].position.x, squads[1].position.x]


func test_lines_that_wrap_meet_in_the_middle_whichever_spawned_first() -> void:
	for kingdom_first in [false, true]:
		var fronts := _meeting(kingdom_first)
		assert_float(fronts[0] + fronts[1]).is_equal_approx(128.0, 0.05)


## Ticks for each wave to reach its route's end: two routes that merge for their last 20
## cells, the short one's wave waiting so both reach the merge together, `long_first`
## spawning the long route's wave first.
func _merging_arrivals(long_first: bool) -> Dictionary:
	var long := PackedVector2Array(
		[Vector2(0, 40), Vector2(60, 40), Vector2(60, 0), Vector2(80, 0)]
	)
	var routes := {
		"short": FormationRoute.new(PackedVector2Array([Vector2(0, 0), Vector2(80, 0)])),
		"long": FormationRoute.new(long),
	}
	var waits := {"short": 60, "long": 0}
	var sim := FormationSimulation.new(4.0, TICK)
	var squads := {}
	for key in ["long", "short"] if long_first else ["short", "long"]:
		squads[key] = sim.spawn_squad(4, _row(4, 10), "player", true, waits[key], routes[key])
	var arrived := {}
	for _i in range(800):
		sim.step()
		for key in squads:
			if not arrived.has(key) and squads[key].state == SkirmishSquad.State.ARRIVED:
				arrived[key] = sim.tick_number()
	return arrived


func test_waves_meeting_on_one_road_queue_by_where_they_stand_not_spawn_order() -> void:
	assert_dict(_merging_arrivals(true)).is_equal(_merging_arrivals(false))


## Three standing groups set to merge, the middle one near enough to meet both at once (spec
## 38 step 1a).
func _three_groups() -> Callable:
	return func():
		var sim := FormationSimulation.new(4.0, TICK)
		for front in [40.0, 42.5, 45.0]:
			var squad := sim.spawn_squad(4, _row(4), "player", true)
			squad.order = SkirmishUnit.Order.HOLD
			squad.state = SkirmishSquad.State.HOLDING
			squad.front_distance = front / 64.0
			squad.fought = true
			squad.merges = true
			FormationMarch.sync_units([squad])
		return sim


func _form_up_step(sim: FormationSimulation, tick: int) -> void:
	FormationGroups.step(sim.squads(), tick + 1, sim.fight_seed, [], 1000)


func test_three_groups_meeting_at_once_merge_the_same_with_their_lists_reversed() -> void:
	var sim: FormationSimulation = _three_groups().call()
	_form_up_step(sim, 0)
	assert_int(sim.squads().size()).is_less(3)  # they did meet
	assert_int(_first_difference(_three_groups(), _form_up_step, 3)).is_equal(0)


## Two drilled player squads retreating, every unit 2 cells out of its place, and a kingdom
## squad close by (spec 38 step 1b): both regroup on one tick, through one hostile index.
func _two_regrouping() -> Callable:
	return func():
		var sim := FormationSimulation.new(1.0, TICK)
		var drilled := _def()
		drilled.discipline = 80  # it meets threats: a retreat backs away facing its foes
		for y in [20.0, 26.0]:
			var lane := FormationRoute.new(PackedVector2Array([Vector2(20, y), Vector2(60, y)]))
			var placements := range(4).map(func(column): return [drilled, Vector2i(0, column)])
			var squad := sim.spawn_squad(4, placements, "player", true, 0, lane)
			squad.order = SkirmishUnit.Order.RETREAT
			for unit in squad.living():
				SquadEntries.loosen(squad, unit, UnitPlaces.of(squad, unit) + Vector2(2, 0))
		var foes := FormationRoute.new(PackedVector2Array([Vector2(24, 23), Vector2(0, 23)]))
		sim.spawn_squad(4, _row(4), "the_kingdom", true, 0, foes)
		return sim


## Where each loose unit of the battle stands, by id.
func _loose(squads: Array) -> String:
	var rows := []
	for squad in squads:
		for unit_id in squad.loose:
			var unit: SkirmishUnit = squad.loose[unit_id]
			var at := SquadEntries.loose_at(squad, unit).snapped(Vector2(0.0001, 0.0001))
			rows.append([unit_id, at, UnitPlaces.bearing_of(squad, unit)])
	rows.sort()
	return str(rows)


func test_two_regrouping_squads_of_one_side_walk_back_the_same_with_their_lists_reversed() -> void:
	var usual: FormationSimulation = _two_regrouping().call()
	var reversed: FormationSimulation = _two_regrouping().call()
	_reverse(reversed.squads())
	for _tick in range(3):
		for sim: FormationSimulation in [usual, reversed]:
			var ctx := {"pace": 0.25, "seconds": TICK, "cells_per_second": 2.5}
			ctx.merge({"seed": sim.fight_seed, "terrain": null})
			var squads := sim.squads()
			ScrumRegroup.step(squads[0].store.field(), CrateRoster.of(squads), ctx)

	assert_str(_loose(usual.squads())).is_not_equal("[]")  # still walking back
	assert_str(_loose(reversed.squads())).is_equal(_loose(usual.squads()))
	assert_str(_state(reversed.squads())).is_equal(_state(usual.squads()))
