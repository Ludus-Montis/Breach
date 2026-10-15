extends GdUnitTestSuite
## SquadMemo: a phase's squad geometry, taken once a squad, gives exactly what the pure
## functions give (SquadGeometry, SquadEdges), and takes it afresh once a squad has moved;
## FormationSight's grid finds a squad exactly at the edge of a unit's range, and not past.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(detection: float = 40.0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.detection_range = detection
	return unit_def


## A block of `count` units `width` wide on a route along `ends`.
func _block(sim: FormationSimulation, ends: Array, count: int, width: int, faction: String):
	var route := FormationRoute.new(PackedVector2Array(ends))
	var placements := []
	for index in range(count):
		placements.append([_def(), Vector2i(index / width, index % width)])
	return sim.spawn_squad(width, placements, faction, true, 0, route)


func test_it_gives_what_the_pure_functions_give_for_turned_squads() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var east = _block(sim, [Vector2(0, 0), Vector2(100, 0)], 12, 4, "player")
	var slant = _block(sim, [Vector2(6, -3), Vector2(60, 50)], 9, 3, "the_kingdom")
	slant.units[4].hp = 0  # a fallen unit counts for neither
	var memo := SquadMemo.new()
	for pair in [[east, slant], [slant, east]]:
		var axis := SquadFrame.lateral_axis(pair[0].heading)
		assert_that(memo.living(pair[1])).is_equal(pair[1].living())
		assert_that(memo.reach(pair[1], axis)).is_equal(SquadEdges.reach(pair[1], axis))
		assert_that(memo.bounds(pair[1])).is_equal(SquadEdges.bounds(pair[1]))
		assert_bool(memo.overlaps(pair[0], pair[1])).is_equal(
			SquadGeometry.overlaps(pair[0], pair[1])
		)
		assert_float(memo.face_gap(pair[0], pair[1])).is_equal(
			SquadEdges.face_gap(pair[0], pair[1])
		)
		assert_bool(memo.overlap_across(pair[0], pair[1])).is_equal(
			SquadEdges.overlap_across(pair[0], pair[1])
		)


func test_a_squad_that_has_moved_has_its_geometry_taken_afresh() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var squad = _block(sim, [Vector2(0, 0), Vector2(100, 0)], 8, 4, "player")
	var memo := SquadMemo.new()
	var before := memo.bounds(squad)
	squad.front_distance += 0.25

	assert_that(memo.bounds(squad)).is_not_equal(before)
	assert_that(memo.bounds(squad)).is_equal(SquadEdges.bounds(squad))


func test_a_squad_at_the_edge_of_range_is_seen_through_the_grid_and_one_past_it_is_not() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var watcher = _block(sim, [Vector2(0, 0), Vector2(0, 100)], 1, 1, "player")
	var other = _block(sim, [Vector2(40, 0), Vector2(40, 100)], 1, 1, "the_kingdom")
	var gap: float = UnitPlaces.of(watcher, watcher.units[0]).distance_to(
		UnitPlaces.of(watcher, other.units[0])
	)
	watcher.units[0].detection = gap
	var memo := SquadMemo.new()

	assert_bool(FormationSight.detects(watcher, other, null, memo)).is_true()
	watcher.units[0].detection = gap - 0.001
	assert_bool(FormationSight.detects(watcher, other, null, memo)).is_false()
