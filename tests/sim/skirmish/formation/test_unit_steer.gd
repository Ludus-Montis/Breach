extends GdUnitTestSuite
## UnitSteer, per Decision 114 (spec 30 round 2): a unit whose straight way to its goal
## runs into a body that won't part for it steps round it on the side nearer its goal, a
## dead-centre tie by a seeded draw; its own squad's bodies, and one on the goal, it leaves
## to the bodies parting.

const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SEED := 4242


func _unit(unit_id: int, squad_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.squad_id = squad_id
	unit.footprint_width = 1
	unit.footprint_depth = 1
	return unit


## A body [point, radius, unit] of another squad's unit.
func _body(at: Vector2, unit_id: int = 2, squad_id: int = 9) -> Array:
	return [at, 0.5, _unit(unit_id, squad_id)]


func test_an_open_way_goes_straight_to_the_goal() -> void:
	var walker := _unit(1, 1)
	var beside := _body(Vector2(3, 2))  # a cell and more clear of the way

	assert_vector(UnitSteer.toward(walker, Vector2.ZERO, Vector2(6, 0), [beside], SEED)).is_equal(
		Vector2(6, 0)
	)


func test_it_steps_round_on_the_side_the_body_leans_away_from() -> void:
	var walker := _unit(1, 1)

	var leans_south := UnitSteer.toward(
		walker, Vector2.ZERO, Vector2(6, 0), [_body(Vector2(2, 0.3))], SEED
	)
	var leans_north := UnitSteer.toward(
		walker, Vector2.ZERO, Vector2(6, 0), [_body(Vector2(2, -0.3))], SEED
	)

	assert_float(leans_south.y).is_less(0.0)  # it passes north of a body south of its way
	assert_float(leans_north.y).is_greater(0.0)
	assert_float(leans_south.x).is_greater(0.0)


func test_a_dead_centre_body_is_passed_on_a_seeded_side_whatever_the_list() -> void:
	var walker := _unit(1, 1)
	var bodies := [_body(Vector2(2, 0), 2), _body(Vector2(2.4, 5), 3)]
	var reversed := bodies.duplicate()
	reversed.reverse()

	var one := UnitSteer.toward(walker, Vector2.ZERO, Vector2(6, 0), bodies, SEED)
	var other := UnitSteer.toward(walker, Vector2.ZERO, Vector2(6, 0), reversed, SEED)

	assert_float(absf(one.y)).is_greater(0.5)
	assert_vector(one).is_equal(other)


func test_its_own_squad_and_a_body_on_the_goal_are_left_to_part() -> void:
	var walker := _unit(1, 1)
	var friend := _body(Vector2(2, 0), 2, 1)
	var on_goal := _body(Vector2(6, 0.2), 3)

	assert_vector(UnitSteer.toward(walker, Vector2.ZERO, Vector2(6, 0), [friend], SEED)).is_equal(
		Vector2(6, 0)
	)
	assert_vector(UnitSteer.toward(walker, Vector2.ZERO, Vector2(6, 0), [on_goal], SEED)).is_equal(
		Vector2(6, 0)
	)


func test_walking_on_it_reaches_its_goal_without_running_into_the_body() -> void:
	var walker := _unit(1, 1)
	var foe := _body(Vector2(3, 0.1))
	var at := Vector2.ZERO
	var nearest := INF
	for _i in range(200):
		at = at.move_toward(UnitSteer.toward(walker, at, Vector2(6, 0), [foe], SEED), 0.1)
		nearest = minf(nearest, at.distance_to(foe[0]))

	assert_vector(at).is_equal_approx(Vector2(6, 0), Vector2(0.0001, 0.0001))
	assert_float(nearest).is_greater_equal(1.0 - 0.02)  # bodies touch at most, never overlap
