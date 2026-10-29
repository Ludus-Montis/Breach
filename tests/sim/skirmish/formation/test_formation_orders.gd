extends GdUnitTestSuite
## The orders' pass in the crate (spec 39 step 8b5-v): the crate builds each order's events
## as data, decoded into the dictionaries FormationEvents made, and reads a retreat's
## discipline from its units' sheets: a ragged retreat's shock is FormationWithdraw's
## disorder, scaled, and its events come in the order they always did.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _def(discipline: int) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 60
	unit_def.items = [WeaponDef.innate_weapon(3)]
	unit_def.speed = 1.0
	unit_def.discipline = discipline
	return unit_def


func _row(unit_def: UnitDef) -> Array:
	var placements := []
	for column in range(8):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


## A head-on fight of 8 v 8, run till they engage: [sim, mine, theirs].
func _fight(mine: UnitDef) -> Array:
	var sim := FormationSimulation.new(2.0, 0.1)
	var me := sim.spawn_squad(8, _row(mine), "player", true)
	var foe := sim.spawn_squad(8, _row(_def(60)), "the_kingdom", false)
	for _i in range(400):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	return [sim, me, foe]


func test_an_order_applied_is_built_by_the_crate_as_the_event_it_always_was() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var me := sim.spawn_squad(8, _row(_def(60)), "player", true)
	sim.order(me.id, SkirmishUnit.Order.HOLD)

	var events := sim.step().filter(func(e): return e["type"] == "order_applied")

	var expected := {"type": "order_applied", "tick": 1, "squad": me.id, "faction": "player"}
	expected["order"] = "HOLD"
	assert_array(events).is_equal([expected])
	assert_str(str(events[0])).is_equal(str(expected))  # its keys in the same order


func test_a_ragged_retreat_is_shocked_by_its_disorder_and_reported_in_order() -> void:
	var setup := _fight(_def(20))
	var sim: FormationSimulation = setup[0]
	var me: SkirmishSquad = setup[1]
	var foe: SkirmishSquad = setup[2]
	me.morale = 100
	var short := FormationWithdraw.disorder(me)
	sim.order(me.id, SkirmishUnit.Order.RETREAT)

	var events := sim.step()

	var shock := roundi(BattleTuning.current().pursuit_ragged_shock * short)
	assert_float(short).is_greater(0.0)
	var kinds := ["order_applied", "disengaged", "withdrawing", "pursuing"]
	var told := events.filter(func(e): return e["type"] in kinds)
	assert_array(told.map(func(e): return e["type"])).is_equal(kinds)
	assert_int(told[3]["squad"]).is_equal(foe.id)
	assert_int(told[3]["of"]).is_equal(me.id)
	assert_str(told[3]["faction"]).is_equal("the_kingdom")
	assert_int(100 - me.morale).is_greater_equal(shock)  # and whatever the fight cost it
