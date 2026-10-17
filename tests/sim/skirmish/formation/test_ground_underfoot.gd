extends GdUnitTestSuite
## GroundBodies.underfoot (spec 30 round 3, part 7): every unit that walks - to its place,
## or loose in a fight - is slowed by the bodies on the ground it steps over, by their
## weight against its own, and blocked by one bodies_ground_block times its mass.

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _body(at: Vector2, size: int) -> SkirmishUnit:
	var body := SkirmishUnit.new()
	_places.stand(body, at)
	body.footprint_width = size
	body.footprint_depth = size
	UnitState.start(body, SkirmishUnit.State.DOWNED)
	return body


func _walker(at: Vector2) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	_places.stand(unit, at)
	return unit


func test_a_body_underfoot_slows_by_its_weight_against_the_walkers() -> void:
	var drag := BattleTuning.current().bodies_ground_drag
	var walker := _walker(Vector2.ZERO)
	var grem := GroundBodies.of_lying([_body(Vector2(0.5, 0), 1)], _places.store.columns)
	var brute := GroundBodies.of_lying([_body(Vector2(1, 0), 2)], _places.store.columns)

	var over_grem := GroundBodies.underfoot(walker, Vector2(0.5, 0), grem)
	assert_float(over_grem).is_equal_approx(1.0 / (1.0 + drag), 0.0001)
	var over_brute := GroundBodies.underfoot(walker, Vector2(0.5, 0), brute)
	assert_float(over_brute).is_equal_approx(1.0 / (1.0 + 4.0 * drag), 0.0001)
	assert_float(GroundBodies.underfoot(walker, Vector2(5, 0), grem)).is_equal(1.0)


func test_a_body_too_heavy_blocks_the_step() -> void:
	var walker := _walker(Vector2.ZERO)
	var huge := GroundBodies.of_lying([_body(Vector2(0.5, 0), 3)], _places.store.columns)

	assert_float(GroundBodies.underfoot(walker, Vector2(0.5, 0), huge)).is_equal(0.0)


func test_a_unit_walking_back_to_its_place_over_a_body_is_slowed() -> void:
	var paces := []
	for with_body in [false, true]:
		var sim := FormationSimulation.new(4.0, 0.1)
		var grem: UnitDef = load("res://content/units/grem.tres")
		var squad := sim.spawn_squad(1, [[grem, Vector2i(0, 0)]], "player", true)
		var unit: SkirmishUnit = squad.units[0]
		var place := FormationWalk.place_of_one(squad, unit)
		var back := -UnitMotion.vector(squad.heading)
		UnitPlaces.put(squad, unit, place + back * 4.0)
		var lying := []
		if with_body:
			var body := _body(Vector2.ZERO, 1)
			body.id = 99
			body.slot = sim.store.enrol(body.id)
			UnitPlaces.put(squad, body, place + back * 3.5)
			lying.append(body)
		var ground := GroundBodies.of_lying(lying, sim.store.columns)
		FormationWalk.walk(squad, unit, place, [0.1, 8.0, ground])
		paces.append(4.0 - UnitPlaces.of(squad, unit).distance_to(place))
	assert_float(paces[1]).is_less(paces[0])
