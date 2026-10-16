extends GdUnitTestSuite
## Reinforcement, per Decision 44 and specs/22-formation-feel-test.md (round 4): a wave
## that reaches a friendly squad in combat joins it from the back - it never adds to the
## front line directly - and a wave that reaches a friendly squad not in combat queues
## behind it rather than passing through.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const ROUTE := 9.0
const TICK := 0.1


func _def(hp: int, dmg: int, speed: float) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = speed
	return unit_def


func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _run(sim: FormationSimulation, done: Callable, limit: int = 4000) -> Array:
	var log := []
	for i in range(limit):
		log.append_array(sim.step())
		if done.call():
			break
	return log


func _players(sim: FormationSimulation) -> Array:
	return sim.squads().filter(func(s): return s.faction_id == "player")


## Two grem waves, the first already locked with a durable militia line.
func _second_wave_behind_a_fight(width: int) -> Array:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var grem := _def(400, 1, 1.0)
	sim.spawn_squad(3, _line(_def(400, 1, 0.8), 3), "the_kingdom", false)
	var first := sim.spawn_squad(3, _line(grem, 3), "player", true)
	_run(sim, func(): return first.state == SkirmishSquad.State.FIGHTING)
	var second := sim.spawn_squad(width, _line(grem, width), "player", true)
	return [sim, first, second]


func test_a_wave_reaching_a_fight_joins_it_from_the_back() -> void:
	var setup := _second_wave_behind_a_fight(3)
	var sim: FormationSimulation = setup[0]
	var first: SkirmishSquad = setup[1]

	var log := _run(sim, func(): return _players(sim).size() == 1)

	assert_int(first.living().size()).is_equal(6)
	(
		assert_array(first.units.slice(3).map(func(u): return UnitPlace.rank(first, u)))
		. is_equal([1, 1, 1])
	)
	assert_array(first.fighters().map(func(u): return UnitPlace.rank(first, u))).is_equal([0, 0, 0])
	assert_int(log.filter(func(e): return e["type"] == "reinforced").size()).is_equal(1)


func test_the_joining_wave_never_engages_the_enemy_itself() -> void:
	var setup := _second_wave_behind_a_fight(3)
	var sim: FormationSimulation = setup[0]
	var second: SkirmishSquad = setup[2]

	var log := _run(sim, func(): return _players(sim).size() == 1)

	var engaged := log.filter(func(e): return e["type"] == "engaged")
	assert_bool(engaged.any(func(e): return e["squad"] == second.id)).is_false()


func test_a_wider_wave_lines_up_on_the_same_centre() -> void:
	var setup := _second_wave_behind_a_fight(5)
	var sim: FormationSimulation = setup[0]
	var first: SkirmishSquad = setup[1]
	var spans_before := first.units.map(
		func(u): return first.lateral_span(u, Vector2.ZERO, UnitPlace.of(first, u))
	)

	_run(sim, func(): return _players(sim).size() == 1)

	(
		assert_array(
			first.units.slice(0, 3).map(
				func(u): return first.lateral_span(u, Vector2.ZERO, UnitPlace.of(first, u))
			)
		)
		. is_equal(spans_before)
	)
	assert_int(first.width).is_equal(5)
	assert_int(first.fighters().size()).is_equal(5)  # its wings step up beside the line


func test_a_wave_reaching_a_holding_squad_queues_behind_it() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var grem := _def(20, 6, 1.0)
	var first := sim.spawn_squad(3, _line(grem, 3), "player", true)
	_run(sim, func(): return first.front_distance > 2.0)
	sim.order(first.id, SkirmishUnit.Order.HOLD)
	var second := sim.spawn_squad(3, _line(grem, 3), "player", true)

	_run(sim, func(): return false, 200)

	assert_int(_players(sim).size()).is_equal(2)
	assert_float(second.front_distance).is_equal_approx(
		first.front_distance - SkirmishSquad.RANK_DEPTH, 0.001
	)
