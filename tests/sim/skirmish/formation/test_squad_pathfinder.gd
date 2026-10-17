extends GdUnitTestSuite
## One pathfinder per group (spec 30): the living unit with the best wits plans the way,
## whoever leads; equal wits go to the battle's seeded draw - never to ids or the order
## the units are listed in (Decision 97).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SquadPathfinder = preload("res://sim/skirmish/formation/squad_pathfinder.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _squad(wits: Array):
	var sim := FormationSimulation.new(2.0, 0.1)
	var placements := []
	for column in wits.size():
		var unit_def := UnitDef.new()
		unit_def.hp = 10
		unit_def.speed = 1.0
		unit_def.items = [WeaponDef.innate_weapon(1)]
		placements.append([unit_def, Vector2i(0, column)])
	var squad = sim.spawn_squad(wits.size(), placements, "player", true)
	for index in wits.size():
		squad.units[index].attributes["wits"] = wits[index]
	return squad


func test_the_wittiest_living_unit_finds_the_way_whoever_leads() -> void:
	var squad = _squad([8, 14, 12])
	squad.units[0].leadership = 50

	assert_object(SquadPathfinder.pick(squad, 1)).is_same(squad.units[1])
	UnitState.put(squad, squad.units[1], SkirmishUnit.State.DEAD)
	assert_object(SquadPathfinder.pick(squad, 1)).is_same(squad.units[2])


func test_equal_wits_go_to_the_seeded_draw_not_the_list_order() -> void:
	var squad = _squad([12, 12, 9])
	var picked := {}
	for fight_seed in range(1, 21):
		var chosen = SquadPathfinder.pick(squad, fight_seed)
		assert_bool(chosen == squad.units[0] or chosen == squad.units[1]).is_true()
		squad.units.reverse()
		UnitState.regather(squad)
		assert_object(SquadPathfinder.pick(squad, fight_seed)).is_same(chosen)
		squad.units.reverse()
		UnitState.regather(squad)
		picked[chosen.id] = true
	assert_int(picked.size()).is_equal(2)


func test_a_squad_with_none_standing_has_no_pathfinder() -> void:
	var squad = _squad([10])
	UnitState.put(squad, squad.units[0], SkirmishUnit.State.DEAD)

	assert_object(SquadPathfinder.pick(squad, 1)).is_null()
