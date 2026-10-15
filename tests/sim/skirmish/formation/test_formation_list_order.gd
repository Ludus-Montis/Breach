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
			units.append([unit.id, unit.hp, unit.rank, unit.column, at, unit.bearing])
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
