extends GdUnitTestSuite
## The scrum's withdrawals, one call into the crate over the scrum's roster
## (FormationWithdraw.step, spec 39 step 8b5-xv): a withdrawing squad safe long enough
## re-forms, its "regrouping" event as FormationEvents made it.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1


func _one() -> Array:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return [[unit_def, Vector2i(0, 0)]]


func test_a_withdrawal_safe_long_enough_re_forms_and_says_so() -> void:
	var sim := FormationSimulation.new(9.0, TICK)
	var mine := sim.spawn_squad(1, _one(), "player", true)
	sim._field.put_squad_withdraw(mine.slot, 1000)  # safe long since
	var ctx := {"tick": 4, "pace": 0.8, "seconds": TICK, "seed": sim.fight_seed}
	ctx["terrain"] = null
	var events := []

	FormationWithdraw.step(sim._field, CrateRoster.of(sim.squads()), ctx, events)

	assert_bool(mine.withdraw.is_empty()).is_true()
	var regrouping := events.filter(func(e): return e["type"] == "regrouping")
	var expected := {"type": "regrouping", "tick": 4, "squad": mine.id, "faction": "player"}
	assert_array(regrouping).is_equal([expected])
