extends GdUnitTestSuite
## Footprints, per Decision 102 and spec 30: a unit's rectangle, its width by depth turned
## to its bearing; the gap between two (so they touch within reach_contact, on a face or a
## corner, turned or not); and how deep two overlap and the shortest way out.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitFootprint = preload("res://sim/skirmish/formation/unit_footprint.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


func _unit(width: int = 1, depth: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.footprint_width = width
	unit.footprint_depth = depth
	return unit


func _gap(here: Vector2, bearing: float, there: Vector2) -> float:
	var mine := UnitFootprint.corners(_unit(), here, bearing)
	return UnitFootprint.gap(mine, UnitFootprint.corners(_unit(), there, 0.0))


func test_a_footprint_is_its_width_across_its_bearing_and_depth_along_it() -> void:
	var brute := UnitFootprint.corners(_unit(2, 1), Vector2(5, 5), 90.0)
	var box := Rect2(brute[0], Vector2.ZERO)
	for corner in brute:
		box = box.expand(corner)

	assert_vector(box.size).is_equal_approx(Vector2(1, 2), Vector2(0.0001, 0.0001))  # facing east
	assert_vector(box.get_center()).is_equal_approx(Vector2(5, 5), Vector2(0.0001, 0.0001))


func test_square_footprints_touch_on_faces_and_corners_but_not_across_a_gap() -> void:
	var here := Vector2(0.5, 0.5)

	assert_float(_gap(here, 90.0, Vector2(1.5, 0.5))).is_equal_approx(0.0, 0.0001)
	assert_float(_gap(here, 90.0, Vector2(1.5, 1.5))).is_equal_approx(0.0, 0.0001)
	assert_float(_gap(here, 90.0, Vector2(2.5, 0.5))).is_greater(
		BattleTuning.current().reach_contact
	)


func test_a_turned_footprint_reaches_along_its_diagonal() -> void:
	var here := Vector2(0.5, 0.5)
	var there := Vector2(1.9, 0.5)  # 0.4 apart, square on

	assert_float(_gap(here, 0.0, there)).is_equal_approx(0.4, 0.0001)
	assert_float(_gap(here, 45.0, there)).is_equal_approx(1.4 - sqrt(0.5) - 0.5, 0.0001)
	assert_float(_gap(here, 45.0, there)).is_less_equal(BattleTuning.current().reach_contact)


func test_an_overlap_is_pushed_out_the_shortest_way() -> void:
	var mine := UnitFootprint.corners(_unit(), Vector2(0.5, 0.5), 0.0)
	var theirs := UnitFootprint.corners(_unit(), Vector2(1.2, 0.6), 0.0)

	var out := UnitFootprint.overlap(mine, theirs)

	assert_float(out[0]).is_equal_approx(0.3, 0.0001)  # 0.3 deep across x, 0.9 across y
	assert_vector(out[1]).is_equal_approx(Vector2(-1, 0), Vector2(0.0001, 0.0001))
	var apart := UnitFootprint.corners(_unit(), Vector2(3, 3), 0.0)
	assert_float(UnitFootprint.overlap(mine, apart)[0]).is_equal(0.0)
