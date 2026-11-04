extends GdUnitTestSuite
## The tick's blows and shots, chosen in the fight's crate pass over the squads' roster
## (FormationMelee.fight, spec 39 steps 8b5-xvii and 8b5-xxi): two units locked in a duel
## each take the other as its target, and each starts again at its melee time, half again
## while its formation wavers. What each blow deals is test_formation_fight.gd's.

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


func _fight(sim: FormationSimulation) -> void:
	FormationMelee.fight(sim.squads(), [10, 11, 7], false, null, sim._field)


func test_units_locked_in_a_duel_take_each_other_as_their_targets() -> void:
	var sim := _duel()
	var units := _units(sim)

	_fight(sim)

	var targets := units.map(func(u): return sim.store.columns.value(u.slot, UnitColumns.TARGET_ID))
	assert_array(targets).is_equal([units[1].id, units[0].id])


func test_a_wavering_formations_units_start_again_half_again_more_slowly() -> void:
	var sim := _duel()
	sim.squads()[1].morale = 20
	var units := _units(sim)

	_fight(sim)

	var cooldowns := units.map(
		func(u): return sim.store.columns.value(u.slot, UnitColumns.ATTACK_COOLDOWN)
	)
	assert_array(cooldowns).is_equal([10, 15])
