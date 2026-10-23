extends GdUnitTestSuite
## How a unit crosses terrain, per spec 30 (Decisions 64 and 85): climbing and swimming are
## movement modes under one rule - the mode's base pace times Decision 64's pair rule, the
## unit's ability (climber N, swimmer N; every unit has both at 0) against the ground's
## demand (a cliff's climb difficulty, water's flows): full at or above it, half one short,
## blocked beyond. A unit loaded past the threshold does neither; one that "sinks" never
## swims, one that "cant_climb" never climbs.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")

const BELOW := Vector2(7.5, 2.5)
const INTO := Vector2(8.5, 2.5)


func _def(carried: float = 0.0, traits: Dictionary = {}) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.speed = 1.0
	unit_def.traits = traits
	if carried > 0.0:
		var pack := ItemDef.new()
		pack.item_name = "pack"
		pack.weight = carried
		unit_def.items = [pack]
	return unit_def


func _water(flows: int) -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(16, 8))
	ground.paint(Rect2i(8, 0, 1, 8), {"depth": 1.2, "flows": flows})
	return ground


func _cliff(climb: int) -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(16, 8))
	ground.paint(Rect2i(8, 0, 8, 8), {"height": 8, "climb": climb})
	return ground


func _pace(ground: FormationTerrain, walker: TerrainWalker) -> float:
	return ground.crossing(walker, BELOW, INTO)


func test_everyone_swims_still_water_at_the_base_swim_pace() -> void:
	var swim := BattleTuning.current().ground_swim_pace

	assert_float(_pace(_water(0), TerrainWalker.of(_def()))).is_equal_approx(swim, 0.0001)
	assert_float(_water(0).factor(1.0, BELOW, INTO)).is_equal(0.0)


func test_flowing_water_meets_the_swimmer_by_the_pair_rule() -> void:
	var swim := BattleTuning.current().ground_swim_pace
	var ground := _water(2)

	assert_float(_pace(ground, TerrainWalker.new(1.0, 0))).is_equal(0.0)
	assert_float(_pace(ground, TerrainWalker.new(1.0, 1))).is_equal_approx(swim * 0.5, 0.0001)
	assert_float(_pace(ground, TerrainWalker.new(1.0, 2))).is_equal_approx(swim, 0.0001)
	assert_float(_pace(ground, TerrainWalker.new(1.0, 5))).is_equal_approx(swim, 0.0001)


func test_a_cliff_meets_the_climber_by_the_pair_rule() -> void:
	var climb := BattleTuning.current().ground_climb_pace
	var ground := _cliff(2)

	assert_float(_pace(ground, TerrainWalker.new(1.0, 0, 0))).is_equal(0.0)
	assert_float(_pace(ground, TerrainWalker.new(1.0, 0, 1))).is_equal_approx(climb * 0.5, 0.0001)
	assert_float(_pace(ground, TerrainWalker.new(1.0, 0, 2))).is_equal_approx(climb, 0.0001)
	assert_float(ground.crossing(TerrainWalker.new(1.0), INTO, BELOW)).is_equal(1.0)


func test_everyone_climbs_a_plain_cliff_one_short_at_half_pace() -> void:
	var climb := BattleTuning.current().ground_climb_pace
	var plain := FormationTerrain.new(Vector2i(16, 8))
	plain.paint(Rect2i(8, 0, 8, 8), {"height": 8})

	assert_int(BattleTuning.current().ground_climb_demand).is_equal(1)
	assert_float(_pace(plain, TerrainWalker.of(_def()))).is_equal_approx(climb * 0.5, 0.0001)
	assert_float(plain.factor(1.0, BELOW, INTO)).is_equal(0.0)


func test_sinks_and_cant_climb_forbid_their_modes() -> void:
	var cart := TerrainWalker.of(_def(0.0, {"sinks": 1, "cant_climb": 1}))

	assert_float(_pace(_water(0), cart)).is_equal(0.0)
	assert_float(_pace(_cliff(0), cart)).is_equal(0.0)
	assert_float(_pace(_cliff(0), TerrainWalker.of(_def(0.0, {"sinks": 1})))).is_greater(0.0)
	assert_float(_pace(_water(0), TerrainWalker.of(_def(0.0, {"cant_climb": 1})))).is_greater(0.0)


func test_a_unit_loaded_past_the_threshold_neither_swims_nor_climbs() -> void:
	var tuning := BattleTuning.current()
	var limit := UnitDef.AVERAGE * tuning.load_per_strength
	var easy := limit * tuning.load_easy
	var threshold := easy + (limit - easy) * tuning.ground_mode_load_share
	var light := TerrainWalker.of(_def(threshold, {"climber": 1}))
	var heavy := TerrainWalker.of(_def(threshold + 0.5, {"climber": 1, "swimmer": 1}))

	assert_float(_pace(_water(0), light)).is_greater(0.0)
	assert_float(_pace(_cliff(1), light)).is_greater(0.0)
	assert_float(_pace(_water(0), heavy)).is_equal(0.0)
	assert_float(_pace(_cliff(1), heavy)).is_equal(0.0)
	assert_int(heavy.climber).is_equal(1)
