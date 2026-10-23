extends GdUnitTestSuite
## Pace on FormationTerrain, per Decision 85 and spec 27 round 4: a squad keeps the pace
## of the worst cell its front rank steps into, and deep water halts a squad that can't
## swim. The ground's own rules (cost, slope, liquid by height, cliffs) are cargo tests
## (native/rust/src/ground/tests.rs, spec 39 step 3a).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1


func _terrain() -> FormationTerrain:
	return FormationTerrain.new(Vector2i(64, 32))


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 20
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return unit_def


func _line(count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([_def(), Vector2i(0, column)])
	return placements


func test_a_squad_half_in_a_wood_slows_as_a_whole() -> void:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.terrain = _terrain()
	sim.terrain.paint(Rect2i(20, 10, 44, 2), {"cost": 0.5})  # under the squad's south half
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var squad := sim.spawn_squad(4, _line(4), "player", true, 0, route)

	for _i in range(40):
		sim.step()  # 25 ticks to the wood's edge at x 20, then 15 at half pace: x 26
	var x := squad.position.x
	assert_float(x).is_less(28.0)
	assert_float(x).is_greater(22.0)


func _crossing(sinks: bool) -> Array:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.terrain = _terrain()
	sim.terrain.paint(Rect2i(20, 0, 2, 32), {"depth": 2.0})
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var squad := sim.spawn_squad(2, _line(2), "player", true, 0, route)
	if sinks:
		for unit in squad.units:
			unit.walker = TerrainWalker.grounded(unit.height)
	var log := []
	for _i in range(160):
		log.append_array(sim.step())
	return [squad, log]


func test_deep_water_across_the_route_halts_a_squad_that_cannot_swim() -> void:
	var crossing := _crossing(true)

	assert_float(crossing[0].position.x).is_less_equal(20.0)
	assert_int(crossing[1].filter(func(e): return e["type"] == "blocked").size()).is_equal(1)


func test_a_squad_that_swims_crosses_deep_water_slowly() -> void:
	# spec 30 round 3: everyone swims, at the swim pace (ground_swim_pace)
	var crossing := _crossing(false)

	assert_float(crossing[0].position.x).is_greater(22.0)
	assert_bool(crossing[1].any(func(e): return e["type"] == "blocked")).is_false()


func test_without_terrain_nothing_changes() -> void:
	var sim := FormationSimulation.new(1.0, TICK)
	var squad := sim.spawn_squad(2, _line(2), "player", true)

	for _i in range(10):
		sim.step()

	assert_float(squad.position.x).is_equal_approx(8.0, 0.0001)
