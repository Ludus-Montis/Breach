extends GdUnitTestSuite
## The tick's opening as one crate call (FormationOpening, spec 39 step 8b5-xxxv): the orders
## queued apply, each squad sees, the fronts and edges lock, the groups find their way and
## the squads move and join, in that order, and GDScript's copies follow. Each pass's own
## scenarios stay with it; the crate's `opening/tests.rs` shows the one call leaves the
## squads, the units and the log as the passes one by one.

const FormationOpening = preload("res://sim/skirmish/formation/formation_opening.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


## A holding squad of three grems of `faction`, its front `front` cells along.
func _squad(sim: FormationSimulation, faction: String, front: float) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in 3:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(3, placements, faction, faction == "player")
	squad.front_distance = front / 64.0
	squad.order = SkirmishUnit.Order.HOLD
	FormationMarch.sync_units([squad])
	return squad


func _open(sim: FormationSimulation, events: Array) -> Dictionary:
	var moving := [FormationSimulation.TRAVEL_SCALE, sim.tick_seconds, sim.route_length]
	return FormationOpening.step(sim.squads(), null, [1, sim.fight_seed], moving, events)


func test_the_orders_queued_apply_and_what_each_squad_sees_comes_back() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var mine := _squad(sim, "player", 40.0)
	var theirs := _squad(sim, "the_kingdom", 60.0)
	sim.order(theirs.id, SkirmishUnit.Order.ADVANCE)
	var events := []

	var seen := _open(sim, events)

	assert_int(theirs.order).is_equal(SkirmishUnit.Order.ADVANCE)
	var applied := events.filter(func(e): return e["type"] == "order_applied")
	assert_array(applied.map(func(e): return e["squad"])).is_equal([theirs.id])
	assert_array(seen.get(mine.id, {}).get("seen", [])).is_equal([theirs])
	assert_bool(seen.get(theirs.id, {}).get("threat", false)).is_true()


func test_an_advancing_squad_moves_in_the_same_call() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var mine := _squad(sim, "player", 10.0)
	mine.order = SkirmishUnit.Order.ADVANCE
	var before := mine.front_distance

	_open(sim, [])

	assert_float(mine.front_distance).is_greater(before)
