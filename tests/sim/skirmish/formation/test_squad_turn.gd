extends GdUnitTestSuite
## SquadTurn, per Decision 74 (specs/27-formations-in-2d.md): a wheel takes as long as the
## outer end needs to march its quarter arc; an about-face is a short pause that puts the
## back rank in front. Reversing the ranks is the crate's (turn/tests.rs); here, ScrumTurn's
## part: the moves under way cancelled and the events.

const SquadTurn = preload("res://sim/skirmish/formation/squad_turn.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, rank: int, column: int, preferred: int = 0) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.set_place(unit, Vector2i(rank, column))
	unit.preferred_position = preferred
	_places.health(unit, 10)
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


func test_an_about_face_cancels_moves_under_way_and_turns_it_about() -> void:
	var members: Array[SkirmishUnit] = [_unit(1, 0, 0), _unit(2, 1, 0)]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, members, null, _places.store)
	squad.swaps.append({"to": {}, "from": {}, "ticks": 3, "total": 5})
	var events := []

	var turned := ScrumTurn.begin(squad, -1, 4, events)

	assert_bool(turned).is_true()
	assert_array(squad.swaps).is_empty()
	assert_int(squad.direction).is_equal(-1)
	assert_int(UnitPlace.of(squad, members[1]).x).is_equal(0)
	assert_array(events.map(func(event): return event["type"])).is_equal(["turning", "turned"])


func test_a_squad_already_facing_its_way_does_not_turn() -> void:
	var members: Array[SkirmishUnit] = [_unit(1, 0, 0)]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, members, null, _places.store)
	var events := []

	assert_bool(ScrumTurn.begin(squad, 1, 4, events)).is_false()
	assert_array(events).is_empty()
