extends GdUnitTestSuite
## WalkModes (spec 30 round 3, part 6): units walk as their own walkers - swimming deep
## water, climbing cliffs - spending stamina a second in a mode, starting none spent,
## recovering none on a climb face or in deep water, and falling from a face spent.

const WalkModes = preload("res://sim/skirmish/formation/walk_modes.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

const SWIMMER := [1.0, 0, 0, true, false]  # height, swimmer, climber, swims, climbs
const CLIMBER := [1.0, 0, 1, false, true]

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(at: Vector2, walker: Array = []) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	_places.stand(unit, at)
	unit.foothold = at
	unit.hp = 20
	unit.max_hp = 20
	if not walker.is_empty():
		unit.walker = TerrainWalker.new(walker[0], walker[1], walker[2], walker[3], walker[4])
	return unit


func _pool() -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(20, 20))
	ground.paint(Rect2i(10, 0, 5, 20), {"depth": 3.0})
	return ground


func _cliff() -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(20, 20))
	ground.paint(Rect2i(10, 0, 10, 20), {"height": 16})  # 4 cells up at x 10
	return ground


func test_a_swimmer_crosses_deep_water_and_a_grounded_unit_does_not() -> void:
	var ground := _pool()
	var swimmer := _unit(Vector2(9.5, 5.5), SWIMMER)
	var walker := _unit(Vector2(9.5, 5.5))

	assert_float(ground.factor(swimmer, _places.at(swimmer), Vector2(10.5, 5.5))).is_greater(0.0)
	assert_float(ground.factor(walker, _places.at(walker), Vector2(10.5, 5.5))).is_equal(0.0)


func test_a_second_swimming_costs_the_swim_rate() -> void:
	var ground := _pool()
	var swimmer := _unit(Vector2(10.5, 5.5), SWIMMER)
	swimmer.stamina = 50.0
	var from := Vector2(10.2, 5.5)

	WalkModes.after_step(swimmer, _places.store.columns, from, ground, 1.0)

	var rate := BattleTuning.current().stamina_swim
	assert_float(swimmer.stamina).is_equal_approx(50.0 - rate, 0.0001)
	assert_bool(swimmer.on_face).is_true()  # no breather in deep water


func test_a_spent_unit_starts_no_swim_but_carries_on_one_it_is_in() -> void:
	var ground := _pool()
	var spent := _unit(Vector2(9.5, 5.5), SWIMMER)
	spent.stamina = 0.0
	spent.max_stamina = 100.0

	assert_bool(WalkModes.may_step(spent, Vector2(9.5, 5.5), Vector2(10.5, 5.5), ground)).is_false()
	assert_bool(WalkModes.may_step(spent, Vector2(11.5, 5.5), Vector2(12.5, 5.5), ground)).is_true()


func test_stepping_onto_a_cliff_begins_a_climb_of_its_height() -> void:
	var ground := _cliff()
	var climber := _unit(Vector2(9.5, 5.5), CLIMBER)
	_places.move(climber, Vector2(10.1, 5.5))

	WalkModes.after_step(climber, _places.store.columns, Vector2(9.5, 5.5), ground, 0.1)

	assert_float(climber.climb_left).is_equal_approx(4.0, 0.0001)
	assert_bool(climber.on_face).is_true()


func test_a_climb_takes_time_and_stamina_at_the_climb_pace() -> void:
	var ground := _cliff()
	var climber := _unit(Vector2(9.5, 5.5), CLIMBER)
	_places.move(climber, Vector2(10.1, 5.5))
	climber.climb_left = 4.0
	climber.stamina = 30.0

	var still := WalkModes.climb(climber, _places.store.columns, ground, 1.0, 1.0)

	var tuning := BattleTuning.current()
	assert_bool(still).is_true()
	assert_float(climber.climb_left).is_equal_approx(4.0 - tuning.ground_climb_pace, 0.0001)
	assert_float(climber.stamina).is_equal_approx(30.0 - tuning.stamina_climb, 0.0001)
	assert_vector(_places.at(climber)).is_equal(Vector2(10.1, 5.5))


func test_a_unit_spent_on_a_climb_face_falls_hurt_by_what_it_climbed() -> void:
	var ground := _cliff()
	var climber := _unit(Vector2(9.5, 5.5), CLIMBER)
	_places.move(climber, Vector2(10.1, 5.5))
	climber.climb_left = 1.0  # three cells up already
	climber.stamina = 0.01

	var still := WalkModes.climb(climber, _places.store.columns, ground, 0.1, 1.0)

	var tuning := BattleTuning.current()
	var climbed := 4.0 - (1.0 - tuning.ground_climb_pace * 0.1)
	var expected := roundi(tuning.fall_damage_per_cell * climbed)
	assert_bool(still).is_false()
	assert_int(climber.hp).is_equal(20 - expected)
	assert_vector(_places.at(climber)).is_equal(Vector2(9.5, 5.5))
	assert_bool(climber.on_face).is_false()
