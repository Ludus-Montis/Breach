extends GdUnitTestSuite
## Routs and the scrum's later phases hang on no list order (Decision 97): who breaks is
## decided before any breaks; routers flee, rally, settle and are struck by what they are
## and where they stand, then by the seeded draw. The feel test's field with both waves
## sent - a line that routs into its reserve - comes out the same with every list reversed.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## Everything an outcome is made of, by id (as test_formation_list_order's).
func _state(squads: Array) -> String:
	var rows := []
	for squad in squads:
		var units := []
		for unit in squad.units:
			var at: Vector2 = UnitPlaces.of(squad, unit).snapped(Vector2(0.0001, 0.0001))
			units.append([unit.id, unit.hp, unit.rank, unit.column, at, unit.bearing])
		units.sort()
		var front := snappedf(squad.front_distance, 0.000001)
		var fight := [squad.engaged_with, squad.flank_contacts.keys(), squad.morale]
		rows.append([squad.id, squad.state, front, squad.facing, fight, units])
	rows.sort()
	return str(rows)


func _reverse(squads: Array) -> void:
	squads.reverse()
	for squad in squads:
		squad.units.reverse()


## The field with both waves built; `waits` sends B to wait in the wood and A after it.
func _field(battle_seed: int) -> FormationField:
	var field := FormationField.new(
		TICK,
		load("res://content/units/grem.tres"),
		8,
		load("res://content/units/kingdom_militia.tres"),
		load("res://content/units/grem_chieftain.tres"),
		null,
		battle_seed
	)
	for _i in range(2000):
		if field.waves["A"].built() == 8 and field.waves["B"].built() == 9:
			break
		field.step()
	return field


## The first tick at which the field and its list-reversed twin differ; 0 if none.
func _first_difference(battle_seed: int, waits: bool, ticks: int) -> int:
	var fields := [_field(battle_seed), _field(battle_seed)]
	for field in fields:
		if waits:
			field.set_wait(true)
			field.send("B")
		else:
			field.send_together(["A", "B"])
	for tick in range(ticks):
		for index in range(2):
			if waits and tick == 90:
				fields[index].send("A")
			if index == 1:
				_reverse(fields[index].sim.squads())
			fields[index].step()
		if _state(fields[0].sim.squads()) != _state(fields[1].sim.squads()):
			return tick + 1
	return 0


func test_a_rout_into_the_reserve_is_the_same_with_its_lists_reversed() -> void:
	assert_int(_first_difference(1, true, 400)).is_equal(0)


func test_both_waves_sent_together_are_the_same_with_their_lists_reversed() -> void:
	assert_int(_first_difference(2, false, 400)).is_equal(0)


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 40
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return unit_def


func _row(columns: int) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([_def(), Vector2i(0, column)])
	return placements


## [breaking squad's state, its friend's state] after one tick: the first breaks at 0
## morale, and its friend nearby, with just enough morale to be shaken to 0 by seeing it,
## is listed after it if `breaking_first`.
func _after_a_break(breaking_first: bool) -> Array:
	var sim := FormationSimulation.new(2.0, TICK)
	var breaking := sim.spawn_squad(4, _row(4), "player", true)
	var friend := sim.spawn_squad(4, _row(4), "player", true, 50)
	breaking.order = SkirmishUnit.Order.HOLD
	breaking.morale = 0
	friend.morale = BattleTuning.current().rout_seen
	if not breaking_first:
		sim.squads().reverse()
	sim.step()
	return [breaking.state, friend.state]


func test_one_break_never_cascades_within_the_tick_for_whoever_is_listed_later() -> void:
	var usual := _after_a_break(true)
	assert_array(_after_a_break(false)).is_equal(usual)
	assert_int(usual[0]).is_equal(SkirmishSquad.State.ROUTING)
	assert_int(usual[1]).is_not_equal(SkirmishSquad.State.ROUTING)
