extends GdUnitTestSuite
## A tick has 100 ms at 10 ticks a second (spec 30 round 3). The small battle - 40 grems
## against 40 militia, 10 wide - takes well under that marching and mid-fight
## (tools/formation_bench.gd); the bounds here are loose, three times the budget, so a
## slower machine passes and only a search gone back to looking at everything fails.

const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")

const BUDGET_MS := 100.0
const SLACK := 3.0


func test_a_small_battle_keeps_well_within_a_ticks_budget() -> void:
	var result := FormationBench.measure(40, {"width": 10, "march": 5, "settle": 10, "fight": 10})

	assert_int(result["contact"]).is_greater(0)
	assert_float(result["march"][0]).is_less(BUDGET_MS * SLACK)
	assert_float(result["fight"][0]).is_less(BUDGET_MS * SLACK)


## Spec 38 step 2's `terrain=mixed`: the clash on ground with a wood and a ford across its
## line of advance; on its way to the fight, some of the grems walk through the wood and
## some of the militia wade the ford.
func test_the_mixed_clash_crosses_the_wood_and_the_ford() -> void:
	var sim := FormationBench.mixed(64, 8, 1)
	var crossed := {"wood": false, "ford": false}
	for _i in range(300):
		sim.step()
		var at: PackedVector2Array = sim.store.columns.column(UnitColumns.POSITION)
		for point in at:  # store: scattered (few: a test's 128 units)
			var cell := Vector2i(point.floor())
			crossed["wood"] = crossed["wood"] or FormationBench.MIXED_WOOD.has_point(cell)
			crossed["ford"] = crossed["ford"] or FormationBench.MIXED_FORD.has_point(cell)

	assert_dict(crossed).is_equal({"wood": true, "ford": true})
	assert_object(sim.terrain).is_not_null()
