extends GdUnitTestSuite
## Movement by facing, per Decisions 95 and 105 and spec 27 round 8: a unit turns freely
## at its turn rate, moves at full speed ahead and at its backward pace straight back, and
## nimbler unit types turn and back away better than heavy ones.

const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

const TICK := 0.1

var _places: UnitPlaces


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(turn_rate: float = 450.0, backward_pace: float = 0.4) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.turn_rate = turn_rate
	unit.backward_pace = backward_pace
	_places.stand(unit, Vector2.ZERO)
	_places.face(unit, 0.0)  # north
	return unit


func _ticks_to_face(unit: SkirmishUnit, wanted: float) -> int:
	for tick in range(1, 100):
		if UnitMotion.turn(unit, _places.store.columns, wanted, TICK):
			return tick
	return 100


func test_a_quarter_turn_takes_two_ticks_and_about_turn_four() -> void:
	assert_int(_ticks_to_face(_unit(), 90.0)).is_equal(2)
	assert_int(_ticks_to_face(_unit(), 180.0)).is_equal(4)
	assert_int(_ticks_to_face(_unit(), 270.0)).is_equal(2)  # the short way round


func test_a_unit_turns_to_any_angle_not_a_step() -> void:
	var unit := _unit()

	var columns := _places.store.columns
	assert_bool(UnitMotion.turn(unit, columns, 30.0, TICK)).is_true()  # within 45 degrees
	assert_float(_places.facing(unit)).is_equal_approx(30.0, 0.0001)
	assert_bool(UnitMotion.turn(unit, columns, 330.0, TICK)).is_false()  # 60 the other way
	assert_float(_places.facing(unit)).is_equal_approx(345.0, 0.0001)
	assert_float(UnitMotion.bearing_to(Vector2.ZERO, Vector2(1, -1), 0.0)).is_equal_approx(
		45.0, 0.0001
	)


func test_pace_falls_from_ahead_to_its_backward_pace() -> void:
	var unit := _unit()

	assert_float(UnitMotion.pace(unit, Vector2(0, -1), 0.0)).is_equal_approx(1.0, 0.001)
	assert_float(UnitMotion.pace(unit, Vector2(1, 0), 0.0)).is_equal_approx(0.7, 0.001)
	assert_float(UnitMotion.pace(unit, Vector2(0, 1), 0.0)).is_equal_approx(0.4, 0.001)


func test_backing_away_keeps_facing_the_foe() -> void:
	var unit := _unit()

	var columns := _places.store.columns
	var at := UnitMotion.walk(
		unit, columns, Vector2(5, 5), Vector2(5, 10), 1.0, TICK, Vector2(5, 0)
	)

	assert_float(_places.facing(unit)).is_equal_approx(0.0, 0.0001)  # still facing north, at the foe
	assert_float(at.y).is_equal_approx(5.4, 0.001)  # at its backward pace


func test_a_grem_is_nimbler_than_a_brute() -> void:
	var grem := FormationUnits.make(load("res://content/units/grem.tres"), Vector2i.ZERO, "p", 1, 1)
	var brute := FormationUnits.make(
		load("res://content/units/grem_brute.tres"), Vector2i.ZERO, "p", 1, 2
	)
	_places.stand(grem, Vector2.ZERO)
	_places.stand(brute, Vector2.ZERO)
	_places.face(grem, 0.0)
	_places.face(brute, 0.0)

	assert_int(_ticks_to_face(grem, 180.0)).is_less(_ticks_to_face(brute, 180.0))
	assert_float(UnitMotion.pace(grem, Vector2(0, 1), 0.0)).is_greater(
		UnitMotion.pace(brute, Vector2(0, 1), 0.0)
	)
