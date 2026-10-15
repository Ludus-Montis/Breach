extends GdUnitTestSuite
## SquadGeometry and SquadEdges reckon from the squad's heading, not the facing nearest it
## (spec 30 round 2, item 1): gaps along it, lines overlapping across it, squads facing off,
## and the edges of a turned frame.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _squad(squad_id: int, at: Vector2, heading: float, width := 2, ranks := 1) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for rank in ranks:
		for column in width:
			var unit := SkirmishUnit.new()
			unit.id = squad_id * 100 + rank * width + column
			_places.set_place(unit, Vector2i(rank, column))
			unit.footprint_depth = 1
			unit.footprint_width = 1
			_places.health(unit, 10)
			members.append(unit)
	var squad := SkirmishSquad.new(squad_id, "player", 1, 0.0, width, members, null, _places.store)
	squad.position = at
	squad.heading = heading
	return squad


func test_squads_face_off_by_how_far_apart_their_headings_are() -> void:
	var from := _squad(1, Vector2.ZERO, 30.0)

	assert_bool(SquadGeometry.facing_off(from, _squad(2, Vector2.ZERO, 200.0))).is_true()
	assert_bool(SquadGeometry.facing_off(from, _squad(2, Vector2.ZERO, 165.0))).is_true()
	assert_bool(SquadGeometry.facing_off(from, _squad(2, Vector2.ZERO, 150.0))).is_false()


func test_the_gap_runs_along_a_slanting_heading() -> void:
	var from := _squad(1, Vector2.ZERO, 45.0)
	var to := _squad(2, Vector2(2, -2), 225.0)

	assert_float(SquadGeometry.gap(from, to) * CELLS).is_equal_approx(2.0 * sqrt(2.0), 0.0001)
	assert_float(SquadGeometry.gap(to, from) * CELLS).is_equal_approx(2.0 * sqrt(2.0), 0.0001)


func test_slanting_lines_overlap_across_their_heading_or_pass_by() -> void:
	var from := _squad(1, Vector2.ZERO, 45.0)
	var across := Vector2(1, 1).normalized()  # from's right hand

	assert_bool(SquadGeometry.overlaps(from, _squad(2, Vector2(3, -3), 225.0))).is_true()
	var beside := _squad(2, Vector2(3, -3) + across * 2.5, 225.0)
	assert_bool(SquadGeometry.overlaps(from, beside)).is_false()
	var crossing := _squad(2, Vector2(3, -3), 135.0)
	assert_bool(SquadGeometry.overlaps(from, crossing)).is_false()


func test_the_edge_met_is_the_quarter_of_the_victims_frame_the_attacker_comes_from() -> void:
	var victim := _squad(1, Vector2.ZERO, 30.0)

	assert_int(SquadEdges.edge_hit(victim, 210.0)).is_equal(SquadEdges.FRONT)
	assert_int(SquadEdges.edge_hit(victim, 30.0)).is_equal(SquadEdges.REAR)
	assert_int(SquadEdges.edge_hit(victim, 120.0)).is_equal(SquadEdges.LEFT)
	assert_int(SquadEdges.edge_hit(victim, 300.0)).is_equal(SquadEdges.RIGHT)
	assert_int(SquadEdges.edge_hit(victim, 50.0)).is_equal(SquadEdges.REAR)


func test_a_turned_squads_edges_are_its_outermost_units_that_way() -> void:
	var squad := _squad(1, Vector2(5, 5), 45.0, 2, 2)  # ids: 100 101 front, 102 103 behind

	assert_array(_ids(SquadEdges.edge_units(squad, SquadEdges.FRONT))).is_equal([100, 101])
	assert_array(_ids(SquadEdges.edge_units(squad, SquadEdges.REAR))).is_equal([102, 103])
	assert_array(_ids(SquadEdges.edge_units(squad, SquadEdges.LEFT))).is_equal([100, 102])
	assert_array(_ids(SquadEdges.edge_units(squad, SquadEdges.RIGHT))).is_equal([101, 103])


func test_the_face_gap_reaches_a_turned_squads_nearest_corner() -> void:
	var from := _squad(1, Vector2.ZERO, 90.0)
	var to := _squad(2, Vector2(6, 0), 315.0)  # its front-left corner points back at from

	assert_float(SquadEdges.face_gap(from, to) * CELLS).is_equal_approx(6.0 - sqrt(0.5), 0.0001)
	assert_bool(SquadEdges.overlap_across(from, to)).is_true()


func _ids(units: Array) -> Array:
	return units.map(func(u): return u.id)
