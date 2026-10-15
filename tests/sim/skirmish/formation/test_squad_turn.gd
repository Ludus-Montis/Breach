extends GdUnitTestSuite
## SquadTurn, per Decision 74 (specs/27-formations-in-2d.md): a wheel takes as long as the
## outer end needs to march its quarter arc; an about-face is a short pause that puts the
## back rank in front.

const SquadTurn = preload("res://sim/skirmish/formation/squad_turn.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, rank: int, column: int, preferred: int = 0) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.rank = rank
	unit.column = column
	unit.preferred_position = preferred
	unit.hp = 10
	return unit


func test_a_sixteen_wide_line_wheels_in_about_one_and_a_half_seconds() -> void:
	# Speed 1 is 8 cells a second; the outer end's quarter arc is pi/2 x 8 cells.
	var ticks := SquadTurn.wheel_ticks(16, 8.0, 0.1)

	assert_int(ticks).is_equal(16)


func test_wider_or_slower_squads_take_longer_to_wheel() -> void:
	assert_int(SquadTurn.wheel_ticks(8, 8.0, 0.1)).is_less(SquadTurn.wheel_ticks(16, 8.0, 0.1))
	assert_int(SquadTurn.wheel_ticks(16, 4.0, 0.1)).is_greater(SquadTurn.wheel_ticks(16, 8.0, 0.1))
	assert_int(SquadTurn.wheel_ticks(1, 1000.0, 0.1)).is_equal(1)


func test_an_about_face_is_a_one_second_pause() -> void:
	assert_int(SquadTurn.about_face_ticks(0.1)).is_equal(10)
	assert_int(SquadTurn.about_face_ticks(2.0)).is_equal(1)


func test_an_about_face_puts_the_back_rank_in_front_and_turns_the_columns() -> void:
	var front := _unit(1, 0, 0)
	var archer := _unit(2, 1, 2, 2)
	var members: Array[SkirmishUnit] = [front, archer]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 3, members, null, _places.store)
	squad.centre_shift = 0.5

	var depth := SquadTurn.reverse_ranks(squad)

	assert_int(depth).is_equal(2)
	assert_int(archer.rank).is_equal(0)
	assert_int(archer.column).is_equal(0)
	assert_int(front.rank).is_equal(1)
	assert_int(front.column).is_equal(2)
	assert_float(squad.centre_shift).is_equal(-0.5)
	assert_bool(squad.reforming).is_true()


func test_an_about_face_cancels_moves_under_way() -> void:
	var members: Array[SkirmishUnit] = [_unit(1, 0, 0), _unit(2, 1, 0)]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, members, null, _places.store)
	squad.swaps.append({"to": {}, "from": {}, "ticks": 3, "total": 5})

	SquadTurn.reverse_ranks(squad)

	assert_array(squad.swaps).is_empty()
