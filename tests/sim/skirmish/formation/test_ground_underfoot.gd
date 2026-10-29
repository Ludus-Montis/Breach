extends GdUnitTestSuite
## Bodies underfoot (spec 30 round 3, part 7): every unit that walks - to its place, or
## loose in a fight - is slowed by the bodies on the ground it steps over. The rule (by
## their weight against its own, blocked by one bodies_ground_block times its mass) is the
## crate's (`lying/tests.rs`, spec 39 step 8a2-ii).

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
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


func test_a_unit_walking_back_to_its_place_over_a_body_is_slowed() -> void:
	var paces := []
	for with_body in [false, true]:
		var sim := FormationSimulation.new(4.0, 0.1)
		var grem: UnitDef = CrateContent.unit("grem")
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
