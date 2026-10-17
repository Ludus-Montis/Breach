extends GdUnitTestSuite
## SkirmishSquad, per specs/22-formation-feel-test.md: a wave in the field - ranks spaced
## behind its front, moving as a block, front-rank fighters and step-up.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(
	unit_id: int, rank: int, column: int, depth: int = 1, width: int = 1, speed: float = 1.0
) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.set_place(unit, Vector2i(rank, column))
	unit.footprint_depth = depth
	unit.footprint_width = width
	_places.pace(unit, speed)
	_places.health(unit, 10)
	return unit


func _squad(width: int, units: Array, direction: int = 1) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	members.assign(units)
	return SkirmishSquad.new(1, "player", direction, 0.0, width, members, null, _places.store)


func test_ranks_sit_behind_the_front_in_the_travel_direction() -> void:
	var squad := _squad(1, [_unit(1, 0, 0), _unit(2, 1, 0)])
	squad.front_distance = 5.0

	assert_float(squad.rank_distance(UnitPlace.rank(squad, squad.units[1]))).is_equal_approx(
		5.0 - SkirmishSquad.RANK_DEPTH, 0.0001
	)
	var facing_back := _squad(1, [_unit(3, 1, 0)], -1)
	facing_back.front_distance = 5.0
	(
		assert_float(facing_back.rank_distance(UnitPlace.rank(facing_back, facing_back.units[0])))
		. is_equal_approx(5.0 + SkirmishSquad.RANK_DEPTH, 0.0001)
	)


func test_a_squad_moves_at_its_slowest_living_units_speed() -> void:
	var slow := _unit(2, 1, 0, 1, 1, 0.5)
	var squad := _squad(1, [_unit(1, 0, 0), slow])

	assert_float(squad.speed()).is_equal(0.5)
	slow.state = SkirmishUnit.State.DEAD
	assert_float(squad.speed()).is_equal(1.0)


func test_fighters_are_the_foremost_living_unit_of_each_column() -> void:
	var brute := _unit(1, 0, 1, 2, 2)
	var squad := _squad(4, [_unit(2, 0, 0), brute, _unit(3, 1, 0), _unit(4, 0, 3), _unit(5, 1, 3)])

	var ids := squad.fighters().map(func(u): return u.id)

	assert_array(ids).contains_exactly_in_any_order([2, 1, 4])


func test_step_up_closes_the_column_after_a_front_death() -> void:
	var front := _unit(1, 0, 0)
	var squad := _squad(1, [front, _unit(2, 1, 0), _unit(3, 2, 0)])
	front.state = SkirmishUnit.State.DEAD

	var moved := squad.compact()

	assert_array(moved.map(func(u): return u.id)).contains_exactly_in_any_order([2, 3])
	assert_int(UnitPlace.rank(squad, squad.units[1])).is_equal(0)
	assert_int(UnitPlace.rank(squad, squad.units[2])).is_equal(1)
	assert_array(squad.fighters().map(func(u): return u.id)).is_equal([2])


func test_a_wide_unit_only_steps_up_when_its_whole_footprint_is_clear() -> void:
	var blocker := _unit(1, 0, 0)
	var squad := _squad(2, [blocker, _unit(2, 0, 1), _unit(3, 1, 0, 1, 2)])
	squad.units[1].state = SkirmishUnit.State.DEAD

	assert_array(squad.compact()).is_empty()  # column 0 is still held at rank 0


func test_a_unit_steps_up_over_passes_and_the_units_behind_follow_while_clear() -> void:
	var lost := [_unit(1, 0, 0), _unit(2, 1, 0)]  # column 0's two front ranks
	var climber := _unit(3, 2, 0)  # two ranks to close: one a pass
	var wide := _unit(6, 3, 0, 1, 2)  # behind it, across columns 0 and 1
	var squad := _squad(2, lost + [climber, _unit(4, 0, 1), _unit(5, 1, 1), wide])
	for unit in lost:
		unit.state = SkirmishUnit.State.DEAD

	var moved := squad.compact()

	assert_array(moved.map(func(u): return u.id)).contains_exactly_in_any_order([3, 6])
	assert_int(UnitPlace.rank(squad, climber)).is_equal(0)
	assert_int(UnitPlace.rank(squad, wide)).is_equal(2)  # column 1's rank 1 still holds it
	assert_int(UnitPlace.rank(squad, squad.units[4])).is_equal(1)


func test_lateral_spans_are_centred_and_mirrored_for_the_other_facing() -> void:
	var squad := _squad(4, [_unit(1, 0, 0)])
	var mirrored := _squad(4, [_unit(2, 0, 0)], -1)

	(
		assert_object(
			squad.lateral_span(squad.units[0], Vector2.ZERO, UnitPlace.of(squad, squad.units[0]))
		)
		. is_equal(Vector2(-2.0, -1.0))
	)
	(
		assert_object(
			mirrored.lateral_span(
				mirrored.units[0], Vector2.ZERO, UnitPlace.of(mirrored, mirrored.units[0])
			)
		)
		. is_equal(Vector2(1.0, 2.0))
	)


func test_no_one_steps_up_into_a_place_a_move_under_way_is_heading_for() -> void:
	var fallen := _unit(1, 0, 0)
	var behind := _unit(2, 1, 0)  # would step up into the hole
	var mover := _unit(3, 1, 1)  # on its way to that same place
	var squad := _squad(3, [fallen, behind, mover, _unit(4, 0, 2)])  # the front still stands
	squad.swaps.append({"to": {mover: Vector2i(0, 0)}, "from": {mover: Vector2i(1, 1)}})
	fallen.state = SkirmishUnit.State.DEAD

	assert_array(squad.compact()).is_empty()  # the hole is the mover's
	assert_int(UnitPlace.rank(squad, behind)).is_equal(1)
	assert_int(UnitPlace.rank(squad, mover)).is_equal(1)  # mid-move: it lands where its move ends
