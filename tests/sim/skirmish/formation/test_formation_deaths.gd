extends GdUnitTestSuite
## The burial as the crate's pass (FormationDeaths.bury, `deaths/bury.rs`, spec 39 step
## 8b5-xxii, whose cargo tests hold the rules): GDScript's copies of the buried units' states
## and wounds follow the crate's, and the pass's events come back as FormationEvents made
## them, a squad at a time: its dead and downed, its fallen leaders, the step-ups with their
## ranks, and its destruction last.

const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


## A unit of 10 hp and constitution 6 at `rank` in its squad's one column.
func _unit(unit_id: int, rank: int, leadership: int = 0) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	unit.courage = 50
	unit.leadership = leadership
	unit.attributes = {"constitution": 6}
	_places.set_place(unit, Vector2i(rank, 0))
	_places.health(unit, 10)
	return unit


## A squad one column wide.
func _squad(squad_id: int, members: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(squad_id, "player", 1, 0.0, 1, typed, null, _places.store)


## The events but "morale_band" (its band is the tuning's), each its type, squad and unit,
## then any rank or leadership.
func _told(events: Array) -> Array:
	var told := events.filter(func(e): return e["type"] != "morale_band")
	return told.map(
		func(e):
			var out := [e["type"], e["squad"], e.get("unit", 0)]
			for extra in ["leadership", "rank"]:
				if e.has(extra):
					out.append(e[extra])
			return out
	)


func test_a_downed_unit_s_state_and_wounds_follow_the_crate_s() -> void:
	var unit := _unit(1, 0)
	var squad := _squad(1, [unit, _unit(2, 1)])
	_places.health(unit, 0)  # felled

	FormationDeaths.bury([squad], 4, [])

	assert_array([unit.state, unit.wounded]).is_equal([SkirmishUnit.State.DOWNED, 1])
	assert_array(squad.living()).has_size(1)


func test_the_events_come_back_a_squad_at_a_time_its_destruction_last() -> void:
	var leader := _unit(1, 0, 2)
	var behind := _unit(2, 1)
	var first := _squad(1, [leader, behind])
	var alone := _unit(3, 0)
	var second := _squad(2, [alone])
	_places.health(alone, -9)  # past its constitution, felled first
	_places.health(leader, -6)
	var events := []

	FormationDeaths.bury([first, second], 4, events)

	var told := [
		["died", 1, 1],
		["leader_fell", 1, 1, 2],
		["stepped_up", 1, 2, 0],
		["died", 2, 3],
		["destroyed", 2, 0],
	]
	assert_array(_told(events)).is_equal(told)
	assert_array([leader.state, alone.state]).is_equal(
		[SkirmishUnit.State.DEAD, SkirmishUnit.State.DEAD]
	)
	assert_int(second.state).is_equal(SkirmishSquad.State.DESTROYED)
