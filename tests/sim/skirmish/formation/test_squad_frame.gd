extends GdUnitTestSuite
## SquadFrame, per Decision 74 (specs/27-formations-in-2d.md): a squad's units placed in
## cells from its front centre and facing, turned rather than mirrored, so a straight
## route reproduces the lane's lateral spans.

const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(rank: int, column: int, depth: int = 1, width: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.footprint_depth = depth
	unit.footprint_width = width
	_places.health(unit, 10)
	_places.stand(unit, Vector2.ZERO)
	_places.set_place(unit, Vector2i(rank, column))
	return unit


## The unit's place in the shared store, for SquadFrame's calls.
func _at(unit: SkirmishUnit) -> Vector2i:
	return UnitPlace.of(_places, unit)


func test_facings_turn_clockwise_with_screen_y_down() -> void:
	assert_vector(SquadFrame.forward(SquadFrame.NORTH)).is_equal(Vector2(0, -1))
	assert_vector(SquadFrame.forward(SquadFrame.EAST)).is_equal(Vector2(1, 0))
	assert_int(SquadFrame.opposite(SquadFrame.NORTH)).is_equal(SquadFrame.SOUTH)
	assert_int(SquadFrame.opposite(SquadFrame.WEST)).is_equal(SquadFrame.EAST)


func test_ranks_lie_behind_the_front_and_columns_to_the_right() -> void:
	# A 4-wide squad facing east with its front centre at (10, 0).
	var first := _unit(0, 0)
	var second := _unit(1, 3)
	var front := SquadFrame.unit_rect(Vector2(10, 0), 90.0, 4, 0.0, first, _at(first))
	var behind := SquadFrame.unit_rect(Vector2(10, 0), 90.0, 4, 0.0, second, _at(second))

	assert_that(front).is_equal(Rect2(9, -2, 1, 1))
	assert_that(behind).is_equal(Rect2(8, 1, 1, 1))


func test_facing_north_turns_the_block_without_mirroring_it() -> void:
	var big := _unit(0, 2, 2, 2)
	var brute := SquadFrame.unit_rect(Vector2(0, 10), 0.0, 4, 0.0, big, _at(big))

	assert_that(brute).is_equal(Rect2(0, 10, 2, 2))


func test_a_straight_route_reproduces_the_lanes_lateral_spans() -> void:
	for direction in [1, -1]:
		var unit := _unit(1, 2, 1, 2)
		var members: Array[SkirmishUnit] = [unit]
		var squad := SkirmishSquad.new(1, "player", direction, 0.0, 5, members, null, _places.store)
		squad.centre_shift = 0.5
		var heading := 90.0 if direction > 0 else 270.0
		var rect := SquadFrame.unit_rect(squad.position, heading, 5, 0.5, unit, _at(unit))

		assert_vector(Vector2(rect.position.y, rect.end.y)).is_equal_approx(
			squad.lateral_span(unit), Vector2(0.0001, 0.0001)
		)


func test_a_turned_frame_turns_its_footprints() -> void:
	# A 2-wide squad heading north-east: its first column's front corner lies half the
	# squad's width to its left (north-west), its rank's depth back south-west.
	var unit := _unit(0, 0)
	var points := SquadFrame.corners(Vector2.ZERO, 45.0, 2, 0.0, unit, _at(unit))
	var half := sqrt(0.5)

	assert_vector(points[0]).is_equal_approx(Vector2(-half, -half), Vector2(0.0001, 0.0001))
	assert_vector(points[2]).is_equal_approx(Vector2(-half, half), Vector2(0.0001, 0.0001))
	var box := SquadFrame.unit_rect(Vector2.ZERO, 45.0, 2, 0.0, unit, _at(unit))
	assert_vector(box.size).is_equal_approx(Vector2(2 * half, 2 * half), Vector2(0.0001, 0.0001))


func test_the_lateral_axis_points_one_way_for_either_heading_on_it() -> void:
	assert_vector(SquadFrame.lateral_axis(0.0)).is_equal(Vector2(1, 0))
	assert_vector(SquadFrame.lateral_axis(180.0)).is_equal(Vector2(1, 0))
	assert_vector(SquadFrame.lateral_axis(90.0)).is_equal(Vector2(0, 1))
	assert_vector(SquadFrame.lateral_axis(270.0)).is_equal(Vector2(0, 1))
	var reach := SquadFrame.extent(
		PackedVector2Array([Vector2(3, 1), Vector2(-2, 4)]), Vector2(1, 0)
	)
	assert_vector(reach).is_equal(Vector2(-2, 3))


func test_a_frame_worked_out_once_places_every_unit_as_place_does() -> void:
	var cases := [
		[Vector2(10.25, -3.5), 37.3, 7, 0.5],
		[Vector2(-4.0, 12.75), 90.0, 32, -1.5],
		[Vector2(0.125, 0.0), 211.9, 5, 0.0],
	]
	var units := [_unit(0, 0), _unit(3, 6), _unit(1, 2, 2, 2), _unit(5, -1), _unit(2, 9, 1, 3)]
	for frame in cases:
		var basis := SquadFrame.basis(frame[0], frame[1], frame[2], frame[3])
		for unit in units:
			var once := SquadFrame.place_in(basis, unit, _at(unit))
			var each := SquadFrame.place(frame[0], frame[1], frame[2], frame[3], unit, _at(unit))
			assert_bool(once == each).override_failure_message("%s != %s" % [once, each]).is_true()
