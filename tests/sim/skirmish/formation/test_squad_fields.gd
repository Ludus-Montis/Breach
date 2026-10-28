extends GdUnitTestSuite
## A squad's remaining fields held by the crate (specs/39-simulation-in-rust.md step 8b5-iv):
## its faction, coded as a unit's; the units that joined it as reinforcements, by id in the
## order they joined; and its living and downed units, which the crate reads from its members
## and the unit store's STATE column, as UnitState keeps them.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _squad(count: int) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for unit_id in range(1, count + 1):
		var unit := SkirmishUnit.new()
		unit.id = unit_id
		members.append(unit)
	return SkirmishSquad.new(1, "player", 1, 0.0, count, members)


func _def(speed: float) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = speed
	return unit_def


func _slots(units: Array) -> Array:
	return units.map(func(u): return u.slot)


func test_a_squads_faction_is_held_by_the_crate() -> void:
	var squad := _squad(1)
	var field: Object = squad.store.field()
	assert_str(field.squad_faction(squad.slot)).is_equal("player")
	squad.faction_id = "the_kingdom"
	assert_str(field.squad_faction(squad.slot)).is_equal("the_kingdom")


func test_units_that_join_are_held_by_their_ids_in_the_order_they_joined() -> void:
	var squad := _squad(3)
	var field: Object = squad.store.field()
	assert_array(Array(field.squad_joined(squad.slot))).is_empty()
	squad.add_joined([squad.units[2], squad.units[0]])
	squad.add_joined([squad.units[1]])
	assert_array(Array(field.squad_joined(squad.slot))).is_equal([3, 1, 2])


func test_a_wave_that_reinforces_a_fight_is_held_as_joined() -> void:
	var sim := FormationSimulation.new(9.0, 0.1)
	var grem := _def(1.0)
	var militia := _def(0.8)
	var line := [[grem, Vector2i(0, 0)], [grem, Vector2i(0, 1)]]
	sim.spawn_squad(2, [[militia, Vector2i(0, 0)], [militia, Vector2i(0, 1)]], "the_kingdom", false)
	var first := sim.spawn_squad(2, line, "player", true)
	for i in range(4000):
		sim.step()
		if first.state == SkirmishSquad.State.FIGHTING:
			break
	sim.spawn_squad(2, line, "player", true)
	for i in range(4000):
		sim.step()
		if not first.joined.is_empty():
			break
	var ids := first.joined.map(func(u): return u.id)
	assert_array(ids).is_not_empty()
	assert_array(Array(first.store.field().squad_joined(first.slot))).is_equal(ids)


func test_its_living_and_downed_are_read_from_its_members_and_their_states() -> void:
	var squad := _squad(5)
	var field: Object = squad.store.field()
	var units := squad.units
	UnitState.put(squad, units[1], SkirmishUnit.State.DEAD)
	UnitState.put(squad, units[2], SkirmishUnit.State.DOWNED)
	UnitState.put(squad, units[3], SkirmishUnit.State.FIGHTING)
	UnitState.put(squad, units[4], SkirmishUnit.State.CARRIED)
	var living := _slots(squad.living())
	assert_array(living).is_equal(_slots([units[0], units[3]]))
	assert_array(Array(field.squad_living(squad.slot))).is_equal(living)
	assert_array(Array(field.squad_downed(squad.slot))).is_equal(_slots(UnitState.downed(squad)))
	assert_array(Array(field.squad_downed(squad.slot))).is_equal(_slots([units[2], units[4]]))
