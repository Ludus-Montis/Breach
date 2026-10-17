extends GdUnitTestSuite
## GroundBodies, per Decision 121 (spec 28 part 6): bodies on the ground slow a marching
## front that steps over them by their weight against its own, and a great one blocks it.

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, at: Vector2, size: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.health(unit, 10)
	_places.stand(unit, at)
	unit.footprint_width = size
	unit.footprint_depth = size
	return unit


## A one-unit squad at the origin marching east, and the lying body given.
func _march(body: SkirmishUnit) -> float:
	var walker: Array[SkirmishUnit] = [_unit(1, Vector2.ZERO)]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, walker, null, _places.store)
	squad.heading = 90.0
	var fallen: Array[SkirmishUnit] = [body]
	var theirs := SkirmishSquad.new(2, "the_kingdom", -1, 0.0, 1, fallen, null, _places.store)
	return GroundBodies.drag(squad, GroundBodies.ground([squad, theirs]))


func test_a_body_its_own_weight_halves_its_pace_and_a_great_one_blocks_it() -> void:
	var grem := _unit(2, Vector2(0.8, 0))
	UnitState.start(grem, SkirmishUnit.State.DOWNED)
	var dragon := _unit(3, Vector2(2.0, 0), 4)
	UnitState.start(dragon, SkirmishUnit.State.DEAD)
	var aside := _unit(4, Vector2(0.5, 3))
	UnitState.start(aside, SkirmishUnit.State.DOWNED)
	var standing := _unit(5, Vector2(0.8, 0))

	assert_float(_march(grem)).is_equal_approx(0.5, 0.0001)
	assert_float(_march(dragon)).is_equal(0.0)
	assert_float(_march(aside)).is_equal(1.0)
	assert_float(_march(standing)).is_equal(1.0)  # a standing one is a body that pushes
