extends GdUnitTestSuite
## FormationRecovery and FormationStrays, per Decisions 121, 125 and 126: wounds lower a
## unit's condition; a poor condition drains HP, standing or downed; a downed unit comes to
## at 1 HP after a seeded while, rejoining its formation if it stands within sight, and
## otherwise makes for home alone; regeneration mends up to its limit, times condition.

const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, at: Vector2) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.hp = 10
	unit.max_hp = 10
	_places.stand(unit, at)
	unit.attributes = {"constitution": 10}
	return unit


func _squad(members: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(1, "player", 1, 0.0, members.size(), typed, null, _places.store)


func _downed(unit: SkirmishUnit) -> SkirmishUnit:
	unit.hp = 0
	unit.state = SkirmishUnit.State.DOWNED
	unit.wounded = 1
	return unit


func _run(squads: Array, seconds: float, events: Array = []) -> Array:
	var strays := []
	for tick in range(roundi(seconds / 0.1)):
		strays.append_array(FormationRecovery.step(squads, 0.1, tick, 3, events))
	return strays


func test_each_wound_lowers_its_condition_past_those_it_is_hardened_to() -> void:
	var step := BattleTuning.current().wounds_condition
	var unit := _unit(1, Vector2.ZERO)
	unit.wounded = 1
	var hardened := _unit(2, Vector2.ZERO)
	hardened.wounded = 1
	hardened.traits = {"hardened": 1}

	assert_float(FormationRecovery.condition_of(unit)).is_equal_approx(1.0 - step, 0.0001)
	assert_float(FormationRecovery.condition_of(hardened)).is_equal(1.0)


func test_a_poor_condition_drains_it_to_death_the_faster_the_worse() -> void:
	var three := _downed(_unit(1, Vector2.ZERO))
	three.wounded = 3  # just below the threshold with the usual tuning
	var four := _downed(_unit(2, Vector2(5, 0)))
	four.wounded = 4
	var two := _downed(_unit(3, Vector2(10, 0)))
	two.wounded = 2  # survivable in otherwise good condition
	var events := []
	_run([_squad([three, four, two])], 5.0, events)

	assert_int(four.hp).is_less(three.hp)
	assert_int(two.hp).is_equal(0)
	_run([_squad([three, four, two])], 25.0, events)
	assert_int(three.state).is_equal(SkirmishUnit.State.DEAD)
	assert_bool(events.any(func(e): return e["type"] == "died_of_wounds")).is_true()


func test_downed_it_comes_to_and_rejoins_its_formation_in_sight() -> void:
	var friend := _unit(2, Vector2(3, 0))
	var downed := _downed(_unit(1, Vector2.ZERO))
	var events := []
	var strays := _run([_squad([friend, downed])], 60.0, events)

	assert_bool(downed.is_alive()).is_true()
	assert_int(downed.hp).is_equal(1)
	assert_int(downed.rank).is_equal(1)  # at the back
	assert_array(strays).is_empty()
	assert_bool(events.any(func(e): return e["type"] == "came_to")).is_true()


func test_with_no_formation_in_sight_it_makes_for_home_alone() -> void:
	var downed := _downed(_unit(1, Vector2.ZERO))
	var strays := _run([_squad([downed])], 60.0)

	assert_int(strays.size()).is_equal(1)
	assert_object(strays[0][0]).is_same(downed)


func test_a_lone_unit_flees_home_and_returns_to_the_reserve() -> void:
	var sim := FormationSimulation.new(1.0, 0.1)
	var grem := UnitDef.new()
	grem.hp = 10
	grem.speed = 1.0
	grem.items = [WeaponDef.innate_weapon(1)]
	var squad := sim.spawn_squad(1, [[grem, Vector2i.ZERO]], "player", true)
	squad.front_distance = 0.5
	sim.step()
	var unit: SkirmishUnit = squad.units[0]
	_downed(unit)
	unit.wake_left = 0.05
	var events := []
	for _i in range(200):
		events.append_array(sim.step())

	assert_bool(events.any(func(e): return e["type"] == "came_to" and e.get("alone"))).is_true()
	assert_bool(events.any(func(e): return e["type"] == "fled_home")).is_true()


func test_regeneration_mends_to_its_max_and_no_further_than_its_limit() -> void:
	var troll := _unit(1, Vector2.ZERO)
	troll.hp = 2
	troll.regeneration = 2.0
	troll.regeneration_left = 5.0
	var squads := [_squad([troll])]
	_run(squads, 3.0)

	assert_int(troll.hp).is_equal(7)  # 5 regained: its limit until it rests
	troll.regeneration_left = 100.0
	_run(squads, 7.0)
	assert_int(troll.hp).is_equal(10)  # never past its max


func test_a_blow_of_a_type_it_fears_stops_its_regeneration_a_while() -> void:
	var troll := _unit(1, Vector2.ZERO)
	troll.hp = 2
	troll.regeneration = 10.0
	troll.regeneration_left = 100.0
	troll.regeneration_stops = ["fire"]
	FormationRecovery.scorch(troll, [[3.0, "slashing", false]], 3)
	assert_float(troll.regeneration_halt).is_equal(0.0)
	FormationRecovery.scorch(troll, [[3.0, "fire", true]], 3)
	_run([_squad([troll])], 1.0)

	assert_int(troll.hp).is_equal(2)


func test_a_downed_regenerator_mends_only_with_the_trait() -> void:
	var plain := _downed(_unit(1, Vector2.ZERO))
	var troll := _downed(_unit(2, Vector2(1, 0)))
	for unit in [plain, troll]:
		unit.regeneration = 10.0
		unit.regeneration_left = 100.0
	troll.traits = {"regenerates_downed": 1}
	_run([_squad([plain, troll])], 0.5)

	assert_int(plain.hp).is_equal(0)
	assert_int(troll.hp).is_greater(0)


func test_it_comes_to_where_it_lies_and_walks_to_the_nearest_friendly_formation() -> void:
	var own_friend := _unit(2, Vector2(30, 0))
	var downed := _downed(_unit(1, Vector2.ZERO))
	var own := _squad([own_friend, downed])
	var bearer := _unit(5, Vector2(2, 0))
	var nearer := _squad([bearer])
	nearer.id = 2
	var events := []
	_run([own, nearer], 60.0, events)

	assert_bool(nearer.units.has(downed)).is_true()  # it joins the nearest, not its own
	assert_bool(own.units.has(downed)).is_false()
	assert_vector(SquadEntries.loose_at(nearer, downed)).is_equal(Vector2.ZERO)  # from where it lay
	assert_int(downed.squad_id).is_equal(2)


func test_the_fallen_lie_where_they_fell() -> void:
	var sim := FormationSimulation.new(1.0, 0.1)
	var grem := UnitDef.new()
	grem.hp = 10
	grem.speed = 1.0
	grem.items = [WeaponDef.innate_weapon(1)]
	var squad := sim.spawn_squad(
		2, [[grem, Vector2i(0, 0)], [grem, Vector2i(0, 1)]], "player", true
	)
	sim.step()
	var fallen: SkirmishUnit = squad.units[0]
	_downed(fallen)
	var where := UnitPlaces.of(squad, fallen)
	for _i in range(20):
		sim.step()

	assert_vector(UnitPlaces.of(squad, fallen)).is_equal(where)
	var marched := UnitPlaces.of(squad, squad.units[1])
	assert_float(marched.x).is_greater(where.x + 1.0)  # its squad marched on


func test_a_wounding_blow_that_downs_adds_its_wounds() -> void:
	var FormationDeaths = load("res://sim/skirmish/formation/formation_deaths.gd")
	var unit := _unit(1, Vector2.ZERO)
	unit.hp = -2
	unit.last_wounding = 2
	FormationDeaths.bury([_squad([unit])], 1, [])

	assert_int(unit.wounded).is_equal(3)


## `count` downed player units due to come to, a kingdom foe standing two cells off each.
func _due_with_foes(count: int, cunning: int = 0) -> Array:
	var downed := []
	var foes := []
	for index in range(count):
		var unit := _downed(_unit(index + 1, Vector2(index * 20, 0)))
		unit.wake_left = 0.05
		unit.traits = {"cunning": cunning}
		downed.append(unit)
		var foe := _unit(100 + index, Vector2(index * 20 + 2, 0))
		foe.faction_id = "the_kingdom"
		foes.append(foe)
	var theirs := _squad(foes)
	theirs.faction_id = "the_kingdom"
	theirs.id = 2
	return [[_squad(downed), theirs], downed, foes]


func test_with_a_foe_near_some_play_dead_and_the_cunning_more_often() -> void:
	var plain := _due_with_foes(40)
	var cunning := _due_with_foes(40, 2)
	_run(plain[0], 0.1)
	_run(cunning[0], 0.1)

	var lying := func(units): return units.filter(func(u): return u.playing_dead).size()
	assert_int(lying.call(plain[1])).is_between(4, 36)
	assert_int(lying.call(cunning[1])).is_greater(lying.call(plain[1]))
	for unit in plain[1]:  # the rest get up into the fight
		assert_bool(unit.playing_dead or unit.is_alive()).is_true()


func test_one_playing_dead_gets_up_once_its_foes_are_gone() -> void:
	var scene := _due_with_foes(40, 5)
	_run(scene[0], 0.1)
	var faker: SkirmishUnit = scene[1].filter(func(u): return u.playing_dead)[0]
	_run(scene[0], 10.0)
	assert_bool(faker.is_alive()).is_false()  # its foe still stands by
	for foe in scene[2]:
		_places.move(foe, _places.at(foe) + Vector2(0, 50))
	_run(scene[0], 5.0)

	assert_bool(faker.is_alive()).is_true()
	assert_bool(faker.playing_dead).is_false()


func test_a_thorough_foe_is_fooled_less_often() -> void:
	var FormationWounds = load("res://sim/skirmish/formation/formation_wounds.gd")
	var fooled := [0, 0]
	for index in range(60):
		var body := _downed(_unit(index + 1, Vector2.ZERO))
		body.playing_dead = true
		for which in range(2):
			var taker := _unit(1000 + index, Vector2(1, 0))
			taker.faction_id = "the_kingdom"
			taker.traits = {"thorough": 2 * which}
			var events := []
			var theirs := _squad([taker])
			theirs.faction_id = "the_kingdom"
			theirs.id = 2
			body.hp = 0
			body.state = SkirmishUnit.State.DOWNED
			FormationWounds.tend([_squad([body]), theirs], 1, 10, 1, events)
			if events.is_empty() and taker.target_id == 0:
				fooled[which] += 1

	assert_int(fooled[0]).is_greater(fooled[1])
	assert_int(fooled[0]).is_between(5, 55)
