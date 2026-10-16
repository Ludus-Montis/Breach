extends GdUnitTestSuite
## Stance and reach reckon from the heading (spec 30 round 2, item 1): a turned line meets
## a threat by turning its own frame a quarter, half or three quarters about, and a unit's
## area is the box round its footprint turned to its squad's heading.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _row(columns: int) -> Array:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.discipline = 60
	var placements := []
	for column in range(columns):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func test_a_slanting_line_turns_its_own_frame_to_meet_a_flank() -> void:
	var sim := FormationSimulation.new(1.0, 0.1)
	var north_east := FormationRoute.new(PackedVector2Array([Vector2(40, 32), Vector2(80, -8)]))
	var line := sim.spawn_squad(6, _row(6), "the_kingdom", true, 0, north_east)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	# Raiders coming south-east onto the line's left (north-west) side.
	var south_east := FormationRoute.new(PackedVector2Array([Vector2(16, 8), Vector2(64, 56)]))
	sim.spawn_squad(2, _row(2), "player", true, 0, south_east)

	var faced := []
	for _i in range(250):
		faced = sim.step().filter(func(e): return e["type"] == "faced")
		if not faced.is_empty():
			break

	assert_bool(faced.is_empty()).is_false()
	assert_float(line.heading).is_equal_approx(45.0, 0.001)
	assert_float(faced[0]["heading"]).is_equal_approx(315.0, 0.001)  # its left, turned
	var outward := Vector2(-1, -1).normalized()  # the new face: north-west of the line
	var face: Vector2 = line.stance["anchor"]
	for unit in line.living():  # its places lie behind the new face, the line turned to it
		var place := ScrumStance.anchor_one(line, unit)
		assert_float((place - face).dot(outward)).is_less_equal(0.0)
		assert_float((place - face).dot(outward)).is_greater(-1.0)


func test_a_units_area_is_its_footprint_turned_to_the_heading() -> void:
	var unit := SkirmishUnit.new()
	unit.footprint_width = 2
	unit.footprint_depth = 1
	_places.health(unit, 10)
	var members: Array[SkirmishUnit] = [unit]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 2, members, null, _places.store)

	squad.heading = 0.0
	assert_vector(ScrumReach.area(squad, unit).size).is_equal(Vector2(2, 1))
	squad.heading = 90.0
	assert_vector(ScrumReach.area(squad, unit).size).is_equal(Vector2(1, 2))
	squad.heading = 45.0
	var half := sqrt(0.5)
	assert_vector(ScrumReach.area(squad, unit).size).is_equal_approx(
		Vector2(3 * half, 3 * half), Vector2(0.0001, 0.0001)
	)
