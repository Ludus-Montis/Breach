extends GdUnitTestSuite
## SkirmishSquad, per specs/22-formation-feel-test.md: a wave in the field - ranks spaced
## behind its front, moving as a block, front-rank fighters. Step-up's scenarios are the
## crate's (`compact/tests.rs`, spec 39 step 5b).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
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
	UnitState.put(squad, slow, SkirmishUnit.State.DEAD)
	assert_float(squad.speed()).is_equal(1.0)


func test_fighters_are_the_foremost_living_unit_of_each_column() -> void:
	var brute := _unit(1, 0, 1, 2, 2)
	var squad := _squad(4, [_unit(2, 0, 0), brute, _unit(3, 1, 0), _unit(4, 0, 3), _unit(5, 1, 3)])

	var ids := squad.fighters().map(func(u): return u.id)

	assert_array(ids).contains_exactly_in_any_order([2, 1, 4])


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


func test_the_crate_holds_the_route_a_squad_follows() -> void:
	var squad := _squad(1, [_unit(1, 0, 0)])
	var field: Object = squad.store.field()
	assert_bool(field.squad_route(squad.slot).is_empty()).is_true()
	var bend := PackedVector2Array([Vector2(0, 0), Vector2(128, 0), Vector2(128, 64)])
	squad.route = FormationRoute.new(bend, 2.0)
	assert_array(Array(field.squad_route(squad.slot))).is_equal(Array(bend))
	squad.route = null
	assert_bool(field.squad_route(squad.slot).is_empty()).is_true()
