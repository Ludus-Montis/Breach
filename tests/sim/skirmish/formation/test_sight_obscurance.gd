extends GdUnitTestSuite
## Sight by obscurance (spec 30), one rule for planning sight: each cell has an obscurance
## (a cell of distance through it costs that much), a sight line sums it cell by cell, and
## sight ends where the sum passes the unit's sight budget. The field's wood obscures by
## the tuning. The sums along a line are cargo tests (native/rust/src/ground/tests.rs, spec
## 39 step 3a); seeing into a wood, fog and planning on what is seen are too
## (native/rust/src/paths/tests.rs, step 3c).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func test_the_fields_wood_obscures_by_the_tuning() -> void:
	var grem: UnitDef = CrateContent.unit("grem")
	var terrain: FormationTerrain = FormationField.new(0.1, grem, 0, grem).sim.terrain
	var wood := Vector2(Rect2i(FormationField.WOOD).get_center()) + Vector2(0.5, 0.5)

	assert_float(terrain.obscurance.at(wood)).is_equal(BattleTuning.current().sight_wood_obscurance)
	assert_float(BattleTuning.current().sight_budget).is_greater(0.0)
	assert_float(PathSight.of(grem, wood, Vector2.RIGHT, terrain).budget).is_equal(
		BattleTuning.current().sight_budget
	)
