extends GdUnitTestSuite
## The fight and its aftermath as one crate call (FormationAftermath.step, `aftermath_step`,
## spec 39 step 8b5-xxxviii): on two units locked in a duel, it leaves GDScript's copies and
## the events as the fight, the burial, the taking, the tending and recovery called one
## after another did; and a router the surrender took is TAKEN in GDScript's copy.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationAftermath = preload("res://sim/skirmish/formation/formation_aftermath.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationTaking = preload("res://sim/skirmish/formation/formation_taking.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
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
## next fight, the kingdom's unit left 3 hp (the next blow fells it), the log emptied.
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
	sim.store.columns.put(line.living()[0].slot, UnitColumns.HP, 3)
	CrateEvents.taken(sim._field)
	return sim


## The five passes one call each, as FormationSimulation made them.
func _phased(sim: FormationSimulation, events: Array) -> Array:
	var squads := sim.squads()
	sim._fight(events)
	FormationDeaths.bury(squads, sim._tick, events)
	FormationTaking.step(squads, sim._tick, TICK, sim.fight_seed, events)
	FormationWounds.tend(squads, sim._tick, 10, sim.fight_seed, events)
	return FormationRecovery.step(squads, TICK, sim._tick, sim.fight_seed, events)


func _whole(sim: FormationSimulation, events: Array) -> Array:
	var ticks := [10, sim.fight_seed, sim._tick]
	return FormationAftermath.step(sim.squads(), ticks, TICK, false, null, events)


## Each unit's state in GDScript's copy and in the crate.
func _states(sim: FormationSimulation) -> Array:
	var out := []
	for squad in sim.squads():
		for unit: SkirmishUnit in squad.units:
			out.append([unit.state, sim.store.columns.value(unit.slot, UnitColumns.STATE)])
	return out


func test_the_one_call_leaves_the_copies_and_events_as_the_passes_one_by_one() -> void:
	var phased := _duel()
	var whole := _duel()
	var events := [[], []]

	var strays := [_phased(phased, events[0]), _whole(whole, events[1])]

	var kinds: Array = events[1].map(func(e): return e["type"])
	assert_bool(kinds.has("died") or kinds.has("downed")).is_true()
	assert_array(events[1]).is_equal(events[0])
	assert_array(_states(whole)).is_equal(_states(phased))
	var fallen: SkirmishUnit = whole.squads()[1].units[0]
	var fell := [SkirmishUnit.State.DOWNED, SkirmishUnit.State.DEAD]
	assert_array(fell).contains([fallen.state])
	assert_int(strays[1].size()).is_equal(strays[0].size())


func test_a_router_the_surrender_took_is_taken_in_gdscripts_copy() -> void:
	var sim := _duel()
	var fleeing: SkirmishSquad = sim.squads()[1]
	var coward: SkirmishUnit = fleeing.living()[0]
	sim.store.columns.put(coward.slot, UnitColumns.HP, 100000)
	coward.courage = 0
	fleeing.state = SkirmishSquad.State.ROUTING
	for tick in range(7, 40):  # a seeded even chance a blow: till it gives up
		sim._tick = tick
		sim.store.columns.put(sim.squads()[0].living()[0].slot, UnitColumns.ATTACK_COOLDOWN, 1)
		_whole(sim, [])
		if not fleeing.living().has(coward):
			break

	assert_int(coward.state).is_equal(SkirmishUnit.State.TAKEN)
	assert_int(sim.store.columns.value(coward.slot, UnitColumns.STATE)).is_equal(coward.state)
