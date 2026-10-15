extends GdUnitTestSuite
## Squads on routes, per Decisions 74 and 75 and spec 27 round 1: a squad keeps its
## distance along its own route and takes its place and facing from it; squads on
## different routes meet through 2D geometry, front to front only.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(hp: int = 20, dmg: int = 2) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = 1.0
	return unit_def


func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _route(points: Array) -> FormationRoute:
	return FormationRoute.new(PackedVector2Array(points))


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var events := []
	for _i in range(ticks):
		events.append_array(sim.step())
	return events


func test_a_squad_follows_its_route_round_a_bend() -> void:
	var sim := FormationSimulation.new(4.0, TICK)
	var bend := _route([Vector2(0, 0), Vector2(32, 0), Vector2(32, 64)])
	var squad := sim.spawn_squad(2, _line(_def(), 2), "player", true, 0, bend)

	assert_int(squad.facing).is_equal(SquadFrame.EAST)
	_run(sim, 50)  # 8 cells a second: 40 cells along, less what the wheel cost it

	assert_float(squad.position.x).is_equal_approx(32.0, 0.01)
	assert_float(squad.position.y).is_between(7.0, 8.0)  # its outer file can't hurry round
	assert_int(squad.facing).is_equal(SquadFrame.SOUTH)
	for unit in squad.living():
		assert_float(UnitPlaces.of(squad, unit).x).is_between(30.0, 34.0)


func test_units_stand_in_their_cells_behind_the_front() -> void:
	var sim := FormationSimulation.new(4.0, TICK)
	var route := _route([Vector2(10, 5), Vector2(80, 5)])
	var squad := sim.spawn_squad(2, _line(_def(), 2), "player", true, 0, route)
	sim.step()

	var front := squad.position
	assert_vector(UnitPlaces.of(squad, squad.living()[0])).is_equal_approx(
		front + Vector2(-0.5, -0.5), Vector2(0.01, 0.01)
	)
	assert_vector(UnitPlaces.of(squad, squad.living()[1])).is_equal_approx(
		front + Vector2(-0.5, 0.5), Vector2(0.01, 0.01)
	)


func test_squads_on_different_routes_meet_front_to_front() -> void:
	var sim := FormationSimulation.new(4.0, TICK)
	var down := _route([Vector2(0, 0), Vector2(20, 0), Vector2(20, 120)])
	var up := _route([Vector2(50, 80), Vector2(20, 80), Vector2(20, -40)])
	var player := sim.spawn_squad(4, _line(_def(), 4), "player", true, 0, down)
	var kingdom := sim.spawn_squad(4, _line(_def(), 4), "the_kingdom", true, 0, up)

	var events := _run(sim, 120)

	var engaged := events.filter(func(e): return e["type"] == "engaged")
	assert_bool(engaged.is_empty()).is_false()
	assert_int(player.facing).is_equal(SquadFrame.SOUTH)
	assert_int(kingdom.facing).is_equal(SquadFrame.NORTH)
	assert_float(absf(kingdom.position.y - player.position.y)).is_less_equal(1.2)


func test_squads_whose_lines_do_not_overlap_pass_each_other() -> void:
	var sim := FormationSimulation.new(4.0, TICK)
	var east := _route([Vector2(0, 0), Vector2(100, 0)])
	var west := _route([Vector2(100, 20), Vector2(0, 20)])
	sim.spawn_squad(4, _line(_def(), 4), "player", true, 0, east)
	sim.spawn_squad(4, _line(_def(), 4), "the_kingdom", true, 0, west)

	var events := _run(sim, 160)

	assert_bool(events.any(func(e): return e["type"] == "engaged")).is_false()
	assert_int(events.filter(func(e): return e["type"] == "arrived").size()).is_equal(2)
