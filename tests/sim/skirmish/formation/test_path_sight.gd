extends GdUnitTestSuite
## A pathfinder's sight from its unit's definition (spec 30): its reach ahead, to the side
## and behind are shares of its detection range. The sight's shape, planning on what is
## seen and rejoining a route are cargo tests (native/rust/src/paths/, spec 39 step 3c).

const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func test_a_units_sight_is_shaped_by_its_definition() -> void:
	var grem := UnitDef.new()
	var shaped := PathSight.of(grem, Vector2.ZERO, Vector2.UP)

	assert_float(shaped.ahead).is_equal(grem.detection_range * grem.sight_ahead)
	assert_float(shaped.side).is_equal(grem.detection_range * grem.sight_side)
	assert_float(shaped.behind).is_equal(grem.detection_range * grem.sight_behind)
	assert_bool(shaped.sees(Vector2(0, -grem.detection_range * grem.sight_ahead + 0.5))).is_true()
