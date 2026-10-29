extends GdUnitTestSuite
## The tick's fight as one crate pass (FormationMelee.fight, `fight_step`, spec 39 step
## 8b5-xix): two units locked in a duel strike each other; each blow hurts its target, says
## "hit" in the crate's log as FormationEvents made it, and tires its striker. A router struck
## may surrender (8b5-xx): TAKEN in GDScript's copy as in the crate, "surrendered" and its
## squad's "destroyed" after the hits.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")

const TICK := 0.1
const HP := 100000


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = HP
	unit_def.speed = 1.0
	unit_def.items = [WeaponDef.innate_weapon(4)]
	return unit_def


## A player unit marching east into a kingdom unit holding, stepped until both fight and a
## second more, so the two touch; then each unit's cooldown set to 1, so it strikes on the
## next fight, and the crate's log emptied.
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
	CrateEvents.taken(sim._field)
	return sim


func _hit(by: SkirmishUnit, on: SkirmishUnit, hp: int) -> Dictionary:
	var hit := {"type": "hit", "tick": 7, "unit": by.id, "faction": by.faction_id}
	hit.merge({"target": on.id, "dmg": 4, "flank": false, "blow": "hit", "target_hp": hp})
	return hit


func test_each_blow_hurts_its_target_says_hit_and_tires_its_striker() -> void:
	var sim := _duel()
	var units: Array = sim.squads().map(func(s): return s.living()[0])
	var hps := units.map(func(u): return sim.store.columns.value(u.slot, UnitColumns.HP))
	var stamina := units.map(func(u): return sim.store.columns.value(u.slot, UnitColumns.STAMINA))

	var taken := FormationMelee.fight(sim.squads(), [10, 11, 7], false, null, sim._field)
	var events := CrateEvents.taken(sim._field)

	assert_array(taken).is_empty()
	var expected := [_hit(units[0], units[1], hps[1] - 4), _hit(units[1], units[0], hps[0] - 4)]
	assert_array(events).is_equal(expected)
	(
		assert_array(events.map(func(e): return e.keys()))
		. is_equal(expected.map(func(e): return e.keys()))
	)
	var blow := BattleTuning.current().stamina_blow
	var left := units.map(func(u): return sim.store.columns.value(u.slot, UnitColumns.STAMINA))
	var expected_left := [stamina[0] - blow * units[0].tiring, stamina[1] - blow * units[1].tiring]
	assert_array(left).is_equal(expected_left)


func test_a_router_struck_surrenders_in_gdscripts_copy_as_in_the_crate() -> void:
	var sim := _duel()
	var fleeing: SkirmishSquad = sim.squads()[1]
	var coward: SkirmishUnit = fleeing.living()[0]
	coward.courage = 0
	fleeing.state = SkirmishSquad.State.ROUTING
	var events := []
	for tick in range(7, 40):  # a seeded even chance a blow: till it gives up
		sim._tick = tick
		sim.store.columns.put(sim.squads()[0].living()[0].slot, UnitColumns.ATTACK_COOLDOWN, 1)
		events = []
		sim._fight(events)
		if not fleeing.living().has(coward):
			break

	assert_int(coward.state).is_equal(SkirmishUnit.State.TAKEN)
	assert_int(sim.store.columns.value(coward.slot, UnitColumns.STATE)).is_equal(coward.state)
	assert_int(fleeing.state).is_equal(SkirmishSquad.State.DESTROYED)
	var tick: int = sim._tick
	var expected := [
		FormationEvents.unit_event("surrendered", tick, fleeing, coward),
		FormationEvents.squad_event("destroyed", tick, fleeing),
	]
	assert_array(events.filter(func(e): return e["type"] != "hit")).is_equal(expected)
	var kinds := events.map(func(e): return e["type"])
	assert_array(kinds).is_equal(["hit", "surrendered", "destroyed"])
