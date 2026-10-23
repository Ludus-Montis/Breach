extends GdUnitTestSuite
## Climbing on stamina (spec 30): there is no breather on a climb face - stamina recovers
## only where a unit can stand. The hardest stretch of a climb is what its stamina must
## last between standable cells. A unit at 0 stamina on a face falls to the foot, hurt by
## the drop and more the heavier it is. Where a unit stands and the plans its stamina
## lasts are cargo tests (native/rust/src/paths/tests.rs, spec 39 step 3c).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const TerrainPaths = preload("res://sim/skirmish/formation/terrain_paths.gd")
const ClimbStamina = preload("res://sim/skirmish/formation/climb_stamina.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


## A staircase of cliffs, a cell a step, up from x 10 to a plateau at x 14-19 on rows 0-9;
## down its east side a cliff to a 2-cell ledge at x 20-21, and another to the ground; a
## wall too high to climb across rows 10-11.
func _stairs() -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(30, 24))
	var columns := {10: 8, 11: 16, 12: 24, 13: 32, 14: 40, 15: 40, 16: 40, 17: 40}
	columns.merge({18: 40, 19: 40, 20: 16, 21: 16})
	for x in columns:
		ground.paint(Rect2i(x, 0, 1, 10), {"height": columns[x]})
	ground.paint(Rect2i(0, 10, 30, 2), {"height": 80, "climb": 9})
	return ground


func _climber(stamina: float) -> TerrainWalker:
	var walker := TerrainWalker.new(1.0, 0, 1)
	walker.stamina = stamina
	return walker


func test_the_hardest_stretch_is_the_most_a_climb_costs_between_ledges() -> void:
	var ground := _stairs()
	var goal := Vector2(16.5, 5.5)
	# each step up: 1 cell at the climb pace (0.25), climber 1 against difficulty 1 - 4 s
	# at 4 stamina a second; the stretch from the foot to the top is 5 steps, 80 stamina
	var fresh := TerrainPaths.cells(ground, _climber(100.0), Vector2(8.5, 5.5), goal)
	var east := TerrainPaths.cells(ground, _climber(20.0), Vector2(25.5, 5.5), goal)

	assert_float(ClimbStamina.hardest(ground, _climber(100.0), fresh)).is_equal_approx(80.0, 0.01)
	assert_float(ClimbStamina.hardest(ground, _climber(20.0), east)).is_equal_approx(16.0, 0.01)


func test_a_spent_unit_on_a_face_falls_and_one_on_a_ledge_does_not() -> void:
	var ground := _stairs()
	var grem := TerrainWalker.new(1.0)

	assert_bool(ClimbStamina.falls(0.0, ground, grem, Vector2(12.5, 5.5))).is_true()
	assert_bool(ClimbStamina.falls(5.0, ground, grem, Vector2(12.5, 5.5))).is_false()
	assert_bool(ClimbStamina.falls(0.0, ground, grem, Vector2(14.5, 5.5))).is_false()


func test_a_fall_hurts_by_the_drop_and_more_the_heavier_the_unit() -> void:
	var ground := _stairs()
	var tuning := BattleTuning.current()
	var foot := Vector2(9.5, 5.5)
	var drop := ClimbStamina.drop(ground, foot, Vector2(12.5, 5.5))  # 24 quarters up
	var grem := UnitDef.new()
	var brute := UnitDef.new()
	brute.footprint_width = 2
	brute.footprint_depth = 2

	assert_float(drop).is_equal_approx(6.0, 0.0001)
	assert_float(ClimbStamina.fall_damage(drop, grem)).is_equal_approx(
		6.0 * tuning.fall_damage_per_cell, 0.0001
	)
	assert_float(ClimbStamina.fall_damage(drop * 2.0, grem)).is_equal_approx(
		2.0 * ClimbStamina.fall_damage(drop, grem), 0.0001
	)
	assert_float(ClimbStamina.fall_damage(drop, brute)).is_greater(
		ClimbStamina.fall_damage(drop, grem)
	)
