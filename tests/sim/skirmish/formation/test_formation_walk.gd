extends GdUnitTestSuite
## FormationWalk (spec 30 round 3, part 2): units walk to their places at their own pace;
## a unit whose place is barred squeezes in and pours through a gap; the frame waits
## while a unit lags beyond its formation's slack (wider the less
## disciplined), but not for one fallen far behind - unless no man is left behind - and
## never under "fall behind, left behind".

const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const PACE := 8.0  # cells a second at speed 1 (FormationSimulation.TRAVEL_SCALE)
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _line(sim: FormationSimulation, count: int) -> SkirmishSquad:
	var grem: UnitDef = load("res://content/units/grem.tres")
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
		assert_vector(UnitPlaces.of(squad, unit)).is_equal(FormationWalk.place_of(squad, unit))


func test_a_unit_pushed_off_its_place_walks_back_at_its_pace() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3)
	var unit: SkirmishUnit = squad.units[0]
	var place := FormationWalk.place_of(squad, unit)
	UnitPlaces.put(squad, unit, place - UnitMotion.vector(squad.heading) * 3.0)  # pushed 3 cells back

	FormationWalk.walk(squad, unit, place, [0.1, PACE])

	var way := place - UnitPlaces.of(squad, unit)
	var stepped := (
		unit.speed * PACE * 0.1 * UnitMotion.pace(unit, way, UnitPlaces.bearing_of(squad, unit))
	)
	assert_float(UnitPlaces.of(squad, unit).distance_to(place)).is_less(3.0)
	assert_float(3.0 - UnitPlaces.of(squad, unit).distance_to(place)).is_less_equal(
		stepped + 0.0001
	)


func test_far_from_its_place_it_faces_its_way_and_near_it_its_formations() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 1)
	var unit: SkirmishUnit = squad.units[0]
	var place := FormationWalk.place_of(squad, unit)
	var across := UnitMotion.vector(squad.heading).orthogonal()
	UnitPlaces.put(squad, unit, place + across * 6.0)
	for _tick in 10:
		FormationWalk.walk(squad, unit, place, [0.1, PACE])
	var facing := UnitPlaces.bearing_of(squad, unit)
	var walking_way := UnitMotion.bearing_to(UnitPlaces.of(squad, unit), place, facing)
	var off := absf(angle_difference(deg_to_rad(facing), deg_to_rad(walking_way)))
	assert_float(off).is_less(0.01)

	for _tick in 40:
		FormationWalk.walk(squad, unit, place, [0.1, PACE])
	assert_vector(UnitPlaces.of(squad, unit)).is_equal_approx(place, Vector2(0.001, 0.001))
	assert_float(UnitPlaces.bearing_of(squad, unit)).is_equal_approx(squad.heading, 0.01)


func test_the_frame_waits_while_a_unit_lags_beyond_its_slack() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3)
	var unit: SkirmishUnit = squad.units[1]
	var place := FormationWalk.place_of(squad, unit)
	var back := -UnitMotion.vector(squad.heading)
	var slack := FormationWalk.slack_of(squad)

	UnitPlaces.put(squad, unit, place + back * (slack * 0.9))
	assert_bool(FormationWalk.waits(squad)).is_false()
	UnitPlaces.put(squad, unit, place + back * (slack + 0.5))
	assert_bool(FormationWalk.waits(squad)).is_true()


func test_a_drilled_formation_keeps_a_tighter_slack() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var loose := _line(sim, 2)
	var drilled := _line(sim, 2)
	for unit in loose.units:
		unit.discipline = 0
	for unit in drilled.units:
		unit.discipline = 100

	var tuning := BattleTuning.current()
	assert_float(FormationWalk.slack_of(loose)).is_equal_approx(tuning.walk_slack_loose, 0.001)
	assert_float(FormationWalk.slack_of(drilled)).is_less(FormationWalk.slack_of(loose))


func test_a_unit_fallen_far_behind_is_waited_for_only_if_no_man_is_left_behind() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 2)
	var unit: SkirmishUnit = squad.units[0]
	var place := FormationWalk.place_of(squad, unit)
	var lost := BattleTuning.current().walk_lost + 1
	UnitPlaces.put(squad, unit, place - UnitMotion.vector(squad.heading) * lost)

	assert_bool(FormationWalk.waits(squad)).is_false()
	for each in squad.units:
		each.traits["no_man_left_behind"] = 1
	assert_bool(FormationWalk.waits(squad)).is_true()


func test_fall_behind_left_behind_never_waits() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 2)
	var unit: SkirmishUnit = squad.units[0]
	UnitPlaces.put(squad, unit, FormationWalk.place_of(squad, unit) + Vector2(0, 3))
	unit.leadership = 1
	unit.traits["fall_behind_left_behind"] = 1

	assert_bool(FormationWalk.waits(squad)).is_false()


func test_a_marching_line_keeps_its_units_on_their_places() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 4)
	squad.order = SkirmishUnit.Order.ADVANCE
	var start := squad.front_distance
	for _tick in 50:
		sim.step()
	assert_float(squad.front_distance).is_greater(start)
	for unit in squad.units:
		var lag := UnitPlaces.of(squad, unit).distance_to(FormationWalk.place_of(squad, unit))
		assert_float(lag).is_less_equal(FormationWalk.slack_of(squad))


func test_a_unit_whose_place_is_barred_squeezes_in_towards_the_centre_line() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var terrain := FormationTerrain.new(Vector2i(64, 32))
	terrain.paint(Rect2i(0, 0, 64, 32), {"depth": 3.0})  # deep water everywhere
	terrain.paint(Rect2i(0, 0, 64, 2), {"depth": 0.0})  # but a 2-cell strip along the route
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 1), Vector2(64, 1)]))
	var squad := _line(sim, 4)
	squad.route = route
	for unit in squad.units:  # ones that can't swim: the water is barred to them
		unit.walker = TerrainWalker.grounded(unit.height)
	squad.front_distance = 20.0 / 64.0
	for unit in squad.units:
		var target := FormationWalk.target_of(squad, unit, terrain)
		assert_float(terrain.factor(unit, target, target)).is_greater(0.0)
		var place := FormationWalk.place_of(squad, unit)
		assert_float(target.x).is_less_equal(place.x + 0.0001)  # across it, or filed in behind


func test_a_unit_barred_straight_ahead_slides_along_the_bank() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var terrain := FormationTerrain.new(Vector2i(64, 32))
	terrain.paint(Rect2i(10, 0, 4, 10), {"depth": 3.0})  # a pool, open to its south
	var squad := _line(sim, 1)
	var unit: SkirmishUnit = squad.units[0]
	UnitPlaces.put(squad, unit, Vector2(9.5, 9.5))
	var goal := Vector2(11.5, 10.5)  # diagonally past the pool's corner
	var before := UnitPlaces.of(squad, unit)
	FormationWalk.walk(squad, unit, goal, [0.1, 8.0], terrain)
	assert_bool(UnitPlaces.of(squad, unit) != before).is_true()
	(
		assert_float(
			terrain.factor(unit.height, UnitPlaces.of(squad, unit), UnitPlaces.of(squad, unit))
		)
		. is_greater(0.0)
	)
