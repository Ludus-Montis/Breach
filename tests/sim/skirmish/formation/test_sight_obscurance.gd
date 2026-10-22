extends GdUnitTestSuite
## Sight by obscurance (spec 30), one rule for planning sight: each cell has an obscurance
## (a cell of distance through it costs that much), a sight line sums it cell by cell, and
## sight ends where the sum passes the unit's sight budget. Woods obscure heavily - a few
## cells in, then nothing - and fog (painted, or field-wide weather) lightly, shortening
## sight rather than blocking it. A pathfinder plans on what it sees by this rule and
## remembers what it has seen, so ground it walked through stays known.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const TerrainPaths = preload("res://sim/skirmish/formation/terrain_paths.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const PathMemory = preload("res://sim/skirmish/formation/path_memory.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const BUDGET := 3.0
const WOOD := {"cost": 0.5, "obscurance": 1.0}


func _eyes(terrain: FormationTerrain, at: Vector2, way: Vector2) -> PathSight:
	return PathSight.new(at, way, 30.0, 30.0, 30.0, terrain, BUDGET)


func test_a_sight_line_sums_obscurance_by_the_distance_through_each_cell() -> void:
	var ground := FormationTerrain.new(Vector2i(40, 20))
	ground.paint(Rect2i(10, 0, 10, 20), {"obscurance": 1.0})
	var seen := ground.obscurance

	assert_float(seen.along(Vector2(2.5, 5.5), Vector2(8.5, 5.5))).is_equal_approx(0.0, 0.0001)
	assert_float(seen.along(Vector2(2.5, 5.5), Vector2(12.5, 5.5))).is_equal_approx(2.5, 0.0001)
	assert_float(seen.along(Vector2(2.5, 5.5), Vector2(30.5, 5.5))).is_equal_approx(10.0, 0.0001)
	var slant := seen.along(Vector2(5.0, 0.0), Vector2(25.0, 20.0))  # 10 cells across at 45
	assert_float(slant).is_equal_approx(10.0 * sqrt(2.0), 0.0001)


func test_a_unit_sees_a_few_cells_into_a_wood_then_nothing() -> void:
	var ground := FormationTerrain.new(Vector2i(40, 20))
	ground.paint(Rect2i(10, 0, 10, 20), WOOD)
	var sight := _eyes(ground, Vector2(5.5, 5.5), Vector2.RIGHT)

	assert_bool(sight.sees(Vector2(9.5, 5.5))).is_true()
	assert_bool(sight.sees(Vector2(12.5, 5.5))).is_true()  # 2.5 cells in
	assert_bool(sight.sees(Vector2(13.5, 5.5))).is_false()  # 3.5 in
	assert_bool(sight.sees(Vector2(25.5, 5.5))).is_false()  # beyond it
	var inside := _eyes(ground, Vector2(15.5, 5.5), Vector2.RIGHT)
	assert_bool(inside.sees(Vector2(17.5, 5.5))).is_true()
	assert_bool(inside.sees(Vector2(19.5, 5.5))).is_false()


func test_fog_shortens_sight_rather_than_blocking_it() -> void:
	var clear := FormationTerrain.new(Vector2i(60, 10))
	var weather := FormationTerrain.new(Vector2i(60, 10))
	weather.obscurance.fog = 0.1
	var bank := FormationTerrain.new(Vector2i(60, 10))
	bank.paint(Rect2i(0, 0, 60, 10), {"obscurance": 0.1})
	var at := Vector2(1.5, 5.5)

	assert_bool(_eyes(clear, at, Vector2.RIGHT).sees(Vector2(29.5, 5.5))).is_true()
	for fogged in [weather, bank]:
		var sight := _eyes(fogged, at, Vector2.RIGHT)
		assert_bool(sight.sees(Vector2(20.5, 5.5))).is_true()
		assert_bool(sight.sees(Vector2(29.5, 5.5))).is_true()
		assert_bool(sight.sees(Vector2(32.5, 5.5))).is_false()
	assert_float(weather.obscurance.at(Vector2(-4.0, 5.0))).is_equal_approx(0.1, 0.0001)


func test_a_sight_line_is_the_same_either_way_and_mirrored() -> void:
	var ground := FormationTerrain.new(Vector2i(30, 30))
	var mirrored := FormationTerrain.new(Vector2i(30, 30))
	for area in [Rect2i(8, 3, 5, 9), Rect2i(17, 14, 6, 4), Rect2i(4, 20, 3, 3)]:
		ground.paint(area, {"obscurance": 0.7})
		var flipped := Rect2i(30 - area.end.x, area.position.y, area.size.x, area.size.y)
		mirrored.paint(flipped, {"obscurance": 0.7})
	var flip := func(at: Vector2) -> Vector2: return Vector2(30.0 - at.x, at.y)
	var lines := [
		[Vector2(1.5, 1.5), Vector2(28.5, 25.5)],
		[Vector2(10.0, 0.0), Vector2(10.0, 29.0)],  # along a grid line
		[Vector2(2.0, 22.0), Vector2(26.0, 2.0)],  # through corners
		[Vector2(15.5, 15.5), Vector2(5.25, 7.75)],
	]
	for line in lines:
		var there: float = ground.obscurance.along(line[0], line[1])
		assert_float(ground.obscurance.along(line[1], line[0])).is_equal_approx(there, 0.0001)
		var back: float = mirrored.obscurance.along(flip.call(line[0]), flip.call(line[1]))
		assert_float(back).is_equal_approx(there, 0.0001)


func test_obscurance_does_not_depend_on_what_was_painted_first() -> void:
	var first := FormationTerrain.new(Vector2i(20, 20))
	first.paint(Rect2i(2, 2, 8, 8), {"cost": 0.5, "obscurance": 1.0})
	first.paint(Rect2i(5, 5, 8, 8), {"height": 4})
	var second := FormationTerrain.new(Vector2i(20, 20))
	second.paint(Rect2i(5, 5, 8, 8), {"height": 4})
	second.paint(Rect2i(2, 2, 8, 8), {"cost": 0.5, "obscurance": 1.0})
	var line := [Vector2(0.5, 0.5), Vector2(19.5, 17.5)]

	assert_float(second.obscurance.along(line[0], line[1])).is_equal(
		first.obscurance.along(line[0], line[1])
	)


func test_the_fields_wood_obscures_by_the_tuning() -> void:
	var grem: UnitDef = CrateContent.unit("grem")
	var terrain: FormationTerrain = FormationField.new(0.1, grem, 0, grem).sim.terrain
	var wood := Vector2(Rect2i(FormationField.WOOD).get_center()) + Vector2(0.5, 0.5)

	assert_float(terrain.obscurance.at(wood)).is_equal(BattleTuning.current().sight_wood_obscurance)
	assert_float(BattleTuning.current().sight_budget).is_greater(0.0)
	assert_float(PathSight.of(grem, wood, Vector2.RIGHT, terrain).budget).is_equal(
		BattleTuning.current().sight_budget
	)


func test_a_plan_cannot_see_through_a_wood_and_remembers_what_it_walked_through() -> void:
	var ground := FormationTerrain.new(Vector2i(40, 20))
	ground.paint(Rect2i(10, 0, 10, 20), WOOD)
	ground.paint(Rect2i(25, 0, 2, 19), {"height": 8, "climb": 3})  # a wall past the wood
	var grem := TerrainWalker.new(1.0)
	var memory := PathMemory.new()
	var start := Vector2(5.5, 5.5)
	var goal := Vector2(35.5, 5.5)

	var outside := TerrainPaths.plan(
		ground, grem, start, goal, _eyes(ground, start, Vector2.RIGHT), memory
	)
	assert_bool(outside["reaches"]).is_false()  # the wall is hidden: straight on into the wood
	assert_bool(outside["cells"].all(func(cell): return cell.y == 5)).is_true()
	var past := Vector2(21.5, 5.5)  # through the wood: the wall comes into view
	TerrainPaths.plan(ground, grem, past, goal, _eyes(ground, past, Vector2.RIGHT), memory)
	var count := ground.size.x * ground.size.y
	assert_bool(memory.knows(5 * ground.size.x + 25, count)).is_true()  # the wall, seen
	assert_bool(memory.knows(5 * ground.size.x + 11, count)).is_true()  # the wood walked in

	var again := TerrainPaths.plan(
		ground, grem, start, goal, _eyes(ground, start, Vector2.RIGHT), memory
	)
	assert_bool(again["reaches"]).is_false()
	assert_int(again["cells"].back().y).is_greater(5)  # heading for the way round the wall


func test_a_plan_through_obscuring_ground_mirrored_is_the_plan_mirrored() -> void:
	var ground := FormationTerrain.new(Vector2i(40, 24))
	var mirrored := FormationTerrain.new(Vector2i(40, 24))
	for area in [Rect2i(12, 3, 6, 12), Rect2i(24, 10, 3, 14)]:
		ground.paint(area, WOOD)
		mirrored.paint(Rect2i(40 - area.end.x, area.position.y, area.size.x, area.size.y), WOOD)
	var start := Vector2(4.5, 8.5)
	var goal := Vector2(35.5, 14.5)
	var flip := func(at: Vector2) -> Vector2: return Vector2(40.0 - at.x, at.y)
	var grem := TerrainWalker.new(1.0)
	var plan := TerrainPaths.plan(ground, grem, start, goal, _eyes(ground, start, Vector2.RIGHT))
	var back := TerrainPaths.plan(
		mirrored,
		grem,
		flip.call(start),
		flip.call(goal),
		_eyes(mirrored, flip.call(start), Vector2.LEFT)
	)

	assert_int(back["cells"].size()).is_equal(plan["cells"].size())
	for index in plan["cells"].size():
		var cell: Vector2i = plan["cells"][index]
		assert_object(back["cells"][index]).is_equal(Vector2i(39 - cell.x, cell.y))
