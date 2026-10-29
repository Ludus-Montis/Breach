extends GdUnitTestSuite
## The tick's blows and shots, chosen in one call into the crate over the squads' roster
## (FormationMelee.blows, spec 39 step 8b5-xvii): two units locked in a duel strike each
## other for their weapons' damage, and each starts again at its melee time, half again
## while its formation wavers.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")

const TICK := 0.1


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 100000
	unit_def.speed = 1.0
	unit_def.items = [WeaponDef.innate_weapon(4)]
	return unit_def


## A player unit marching east into a kingdom unit holding, stepped until both fight and a
## second more, so the two touch; then each unit's cooldown set to 1, so it strikes on the
## next choice.
func _duel() -> FormationSimulation:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.fight_seed = 11
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var west := FormationRoute.new(PackedVector2Array([Vector2(40, 10), Vector2(0, 10)]))
	sim.spawn_squad(1, [[_def(), Vector2i(0, 0)]], "player", true, 0, east)
	var line := sim.spawn_squad(1, [[_def(), Vector2i(0, 0)]], "the_kingdom", true, 0, west)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	for _i in range(600):
		sim.step()
		if sim.squads().all(func(s): return s.state == SkirmishSquad.State.FIGHTING):
			break
	for _i in range(10):
		sim.step()
	for squad in sim.squads():
		for unit in squad.living():
			sim.store.columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, 1)
	return sim


func _units(sim: FormationSimulation) -> Array:
	return sim.squads().map(func(s): return s.living()[0])


func test_units_locked_in_a_duel_strike_each_other_for_their_weapons_damage() -> void:
	var sim := _duel()
	var units := _units(sim)

	var chosen := FormationMelee.blows(sim.squads(), 10, 11, sim._field)

	var melee: Array = chosen[0].map(func(b): return [b[0].id, b[1].id, b[2], b[3]])
	var expected := [[units[0].id, units[1].id, 4, false], [units[1].id, units[0].id, 4, false]]
	assert_array(melee).is_equal(expected)
	assert_array(chosen[1]).is_empty()
	var targets := units.map(func(u): return sim.store.columns.value(u.slot, UnitColumns.TARGET_ID))
	assert_array(targets).is_equal([units[1].id, units[0].id])


func test_a_wavering_formations_units_start_again_half_again_more_slowly() -> void:
	var sim := _duel()
	sim.squads()[1].morale = 20
	var units := _units(sim)

	FormationMelee.blows(sim.squads(), 10, 11, sim._field)

	var cooldowns := units.map(
		func(u): return sim.store.columns.value(u.slot, UnitColumns.ATTACK_COOLDOWN)
	)
	assert_array(cooldowns).is_equal([10, 15])
