extends GdUnitTestSuite
## Spec 27 round 6 fixes, from the round 5 feel test: a partial wave is centred on its
## route; squads whose units come within reach fight, whatever their faces; a waiting wave
## ignores waves on its own route; a squad closes ranks over its dead; and a led line
## turns only towards an enemy closing in, not one holding its ground.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SquadRanks = preload("res://sim/skirmish/formation/squad_ranks.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _def(hp: int, dmg: int, leadership: int = 0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = 1.0
	unit_def.build_seconds = 0.1
	unit_def.leadership = leadership
	return unit_def


func _row(unit_def: UnitDef, columns: int) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _run(step: Callable, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(step.call())
	return log


func _of(log: Array, kind: String) -> Array:
	return log.filter(func(e): return e["type"] == kind)


## A sim, and a kingdom line 4 wide facing west, its front at x 40
## (it covers x 40 to 41, y 30 to 34).
func _with_line(led: int = 0) -> Array:
	var sim := FormationSimulation.new(1.0, TICK)
	var route := FormationRoute.new(PackedVector2Array([Vector2(40, 32), Vector2(0, 32)]))
	var line := sim.spawn_squad(4, _row(_def(400, 2, led), 4), "the_kingdom", true, 0, route)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	return [sim, line]


func test_a_partial_wave_is_centred_on_its_route() -> void:
	var field := FormationField.new(TICK, _def(20, 3), 1, _def(60, 2))
	_run(field.step, 3)

	var wave := field.send("B")

	assert_int(wave.living().size()).is_equal(1)
	field.step()
	assert_float(UnitPlaces.of(wave, wave.living()[0]).y).is_equal_approx(FormationField.B_Y, 0.01)


func test_a_squad_passing_beside_a_line_fights_it() -> void:
	var setup := _with_line()
	var sim: FormationSimulation = setup[0]
	var route := FormationRoute.new(PackedVector2Array([Vector2(42.5, 0), Vector2(42.5, 64)]))
	var passer := sim.spawn_squad(1, _row(_def(400, 2), 1), "player", true, 0, route)

	var log := _run(sim.step, 200)

	assert_bool(_of(log, "engaged").is_empty()).is_false()
	assert_bool(_of(log, "hit").is_empty()).is_false()
	assert_float(passer.position.y).is_less(40.0)  # held by the fight, not walked past


func test_a_waiting_wave_ignores_a_wave_on_its_own_route() -> void:
	var field := FormationField.new(TICK, _def(20, 3), 8, _def(60, 2))
	field.set_wait(true)
	_run(field.step, 3)
	var first := field.send("B")
	_run(field.step, 3)
	var second := field.send("B")

	_run(field.step, 300)

	assert_bool(first.staging.is_empty()).is_false()
	assert_bool(second.staging.is_empty()).is_false()


func test_a_squad_closes_ranks_over_its_dead() -> void:
	var units: Array[SkirmishUnit] = []
	for place in [Vector2i(0, 0), Vector2i(0, 2), Vector2i(1, 1), Vector2i(1, 2)]:
		var unit := SkirmishUnit.new()
		unit.id = units.size() + 1
		unit.rank = place.x
		unit.column = place.y
		units.append(unit)
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 3, units, null, _places.store)

	SquadRanks.close(squad)

	var places := units.map(func(u): return Vector2i(u.rank, u.column))
	assert_array(places).is_equal([Vector2i(0, 0), Vector2i(0, 1), Vector2i(0, 2), Vector2i(1, 1)])


func test_a_led_line_does_not_turn_towards_an_enemy_holding_its_ground() -> void:
	var setup := _with_line(2)
	var sim: FormationSimulation = setup[0]
	var route := FormationRoute.new(PackedVector2Array([Vector2(41, 20), Vector2(41, 64)]))
	var still := sim.spawn_squad(2, _row(_def(400, 2), 2), "player", true, 0, route)
	sim.order(still.id, SkirmishUnit.Order.HOLD)

	var log := _run(sim.step, 100)

	assert_int(_of(log, "faced").size()).is_equal(0)
