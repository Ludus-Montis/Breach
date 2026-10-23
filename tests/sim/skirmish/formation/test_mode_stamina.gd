extends GdUnitTestSuite
## Stamina in a movement mode (spec 30): a step's mode - walking, climbing a cliff,
## swimming water at least the unit's height deep - and the stamina it costs, per second
## in the mode, not per cell: at half pace a step takes twice as long and costs twice as
## much ("climb 1 speed 2 could move 2 up a difficulty 1 surface in 1 movement so
## 1 x stam-cost, climb 0 speed 2 would take 2 movements so 2 stam-cost"). That a spent
## unit can't start a climb or a swim is the crate's march's (march/tests.rs, spec 39 step
## 3d).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const ModeStamina = preload("res://sim/skirmish/formation/mode_stamina.gd")

const BELOW := Vector2(7.5, 2.5)
const INTO := Vector2(8.5, 2.5)


func _ground() -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(16, 8))
	ground.paint(Rect2i(8, 0, 1, 4), {"height": 8})  # a cliff, climb difficulty 1
	ground.paint(Rect2i(8, 4, 1, 2), {"depth": 1.2})  # deep water
	ground.paint(Rect2i(8, 6, 1, 2), {"depth": 0.4})  # a ford
	return ground


func test_a_step_is_walked_climbed_or_swum_by_the_ground() -> void:
	var ground := _ground()
	var grem := TerrainWalker.new(1.0)

	assert_int(ground.mode(grem, BELOW, INTO)).is_equal(TerrainWalker.Mode.CLIMBING)
	assert_int(ground.mode(grem, INTO, BELOW)).is_equal(TerrainWalker.Mode.WALKING)
	assert_int(ground.mode(grem, Vector2(7.5, 4.5), Vector2(8.5, 4.5))).is_equal(
		TerrainWalker.Mode.SWIMMING
	)
	assert_int(ground.mode(grem, Vector2(7.5, 6.5), Vector2(8.5, 6.5))).is_equal(
		TerrainWalker.Mode.WALKING
	)
	assert_int(ground.mode(TerrainWalker.new(2.0), Vector2(7.5, 4.5), Vector2(8.5, 4.5))).is_equal(
		TerrainWalker.Mode.WALKING
	)


func test_stamina_is_paid_per_second_so_half_pace_costs_twice() -> void:
	var tuning := BattleTuning.current()
	var ground := _ground()
	var able := TerrainWalker.new(1.0, 0, 1)
	var short := TerrainWalker.new(1.0, 0, 0)
	var seconds := 1.0 / (2.0 * tuning.ground_climb_pace)

	var full := ModeStamina.step_cost(ground, able, BELOW, INTO, 2.0)
	assert_float(full).is_equal_approx(seconds * tuning.stamina_climb, 0.0001)
	assert_float(ModeStamina.step_cost(ground, short, BELOW, INTO, 2.0)).is_equal_approx(
		2.0 * full, 0.0001
	)
	(
		assert_float(ModeStamina.step_cost(ground, able, Vector2(1.5, 1.5), Vector2(2.5, 1.5), 2.0))
		. is_equal(0.0)
	)
	var swum := ModeStamina.step_cost(ground, able, Vector2(7.5, 4.5), Vector2(8.5, 4.5), 1.0)
	assert_float(swum).is_equal_approx(tuning.stamina_swim / tuning.ground_swim_pace, 0.0001)
