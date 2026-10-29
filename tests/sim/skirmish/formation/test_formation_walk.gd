extends GdUnitTestSuite
## FormationWalk (spec 30 round 3, part 2) through a battle: a placed squad stands on its
## places, a marching line keeps its units on them, and its frame slows for one lagging.
## The walk, the slack, the wait and the squeeze are the crate's march's (march/tests.rs,
## spec 39 step 3d); the share reads each unit where it lives (moving/share/tests.rs, 8b5-ix).

const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _line(sim: FormationSimulation, count: int) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, "player", true)
	squad.order = SkirmishUnit.Order.HOLD
	return squad


func test_a_placed_squad_stands_on_its_places() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3)
	for unit in squad.units:
		assert_vector(UnitPlaces.of(squad, unit)).is_equal(FormationWalk.place_of_one(squad, unit))


func test_a_marching_line_keeps_its_units_on_their_places() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 4)
	squad.order = SkirmishUnit.Order.ADVANCE
	var start := squad.front_distance
	for _tick in 50:
		sim.step()
	assert_float(squad.front_distance).is_greater(start)
	for unit in squad.units:
		var lag := UnitPlaces.of(squad, unit).distance_to(FormationWalk.place_of_one(squad, unit))
		assert_float(lag).is_less_equal(FormationWalk.slack_of(squad))


func test_a_unit_lagging_three_quarters_of_its_slack_halves_its_frames_step() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3)
	assert_float(FormationWalk.share(squad)).is_equal(1.0)
	var lagging: SkirmishUnit = squad.units[1]
	var behind := Vector2.from_angle(deg_to_rad(squad.heading - 90.0)) * -1.0
	var lag := behind * FormationWalk.slack_of(squad) * 0.75
	UnitPlaces.put(squad, lagging, UnitPlaces.of(squad, lagging) + lag)
	assert_float(FormationWalk.share(squad)).is_equal_approx(0.5, 0.0001)
