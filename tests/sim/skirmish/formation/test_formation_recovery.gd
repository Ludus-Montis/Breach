extends GdUnitTestSuite
## FormationRecovery and FormationStrays, per Decisions 121, 125 and 126, as GDScript lands
## what the crate decides (`recovery.rs`, whose tests hold the rules): a downed unit comes to
## at 1 HP after a seeded while, rejoining the nearest formation in sight at its back, and
## otherwise makes for home alone; a unit whose regeneration a blow stopped mends nothing; the
## fallen lie where they fell and take their last blow's wounds.

const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
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
	_places.health(unit, 10)
	unit.max_hp = 10
	_places.stand(unit, at)
	unit.attributes = {"constitution": 10}
	return unit


func _squad(members: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(1, "player", 1, 0.0, members.size(), typed, null, _places.store)


## Downs `unit`; `squad` is the one it is in, if it is in one yet.
func _downed(unit: SkirmishUnit, squad: SkirmishSquad = null) -> SkirmishUnit:
	_places.health(unit, 0)
	if squad != null:
		UnitState.put(squad, unit, SkirmishUnit.State.DOWNED)
	else:
		UnitState.start(unit, SkirmishUnit.State.DOWNED)
	unit.wounded = 1
	return unit


func _run(squads: Array, seconds: float, events: Array = []) -> Array:
	var strays := []
	for tick in range(roundi(seconds / 0.1)):
		strays.append_array(FormationRecovery.step(squads, 0.1, tick, 3, events))
	return strays


func test_downed_it_comes_to_and_rejoins_its_formation_in_sight() -> void:
	var friend := _unit(2, Vector2(3, 0))
	var downed := _downed(_unit(1, Vector2.ZERO))
	var events := []
	var strays := _run([_squad([friend, downed])], 60.0, events)

	assert_bool(downed.is_alive()).is_true()
	assert_int(UnitHealth.of_one(_places, downed)).is_equal(1)
	assert_int(UnitPlace.of(_places, downed).x).is_equal(1)  # at the back
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
	_downed(unit, squad)
	squad.store.columns.put(unit.slot, UnitColumns.WAKE_LEFT, 0.05)
	var events := []
	for _i in range(200):
		events.append_array(sim.step())

	assert_bool(events.any(func(e): return e["type"] == "came_to" and e.get("alone"))).is_true()
	assert_bool(events.any(func(e): return e["type"] == "fled_home")).is_true()


func test_a_unit_whose_regeneration_a_blow_stopped_mends_nothing_a_while() -> void:
	var troll := _unit(1, Vector2.ZERO)
	_places.health(troll, 2)
	troll.regeneration = 10.0
	_places.store.columns.put(troll.slot, UnitColumns.REGENERATION_LEFT, 100.0)
	_places.store.columns.put(troll.slot, UnitColumns.REGENERATION_HALT, 5.0)  # landing.rs's
	_run([_squad([troll])], 1.0)

	assert_int(UnitHealth.of_one(_places, troll)).is_equal(2)


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
	_downed(fallen, squad)
	var where := UnitPlaces.of(squad, fallen)
	for _i in range(20):
		sim.step()

	assert_vector(UnitPlaces.of(squad, fallen)).is_equal(where)
	var marched := UnitPlaces.of(squad, squad.units[1])
	assert_float(marched.x).is_greater(where.x + 1.0)  # its squad marched on


func test_a_wounding_blow_that_downs_adds_its_wounds() -> void:
	var FormationDeaths = load("res://sim/skirmish/formation/formation_deaths.gd")
	var unit := _unit(1, Vector2.ZERO)
	_places.health(unit, -2)
	_places.store.columns.put(unit.slot, UnitColumns.LAST_WOUNDING, 2)
	FormationDeaths.bury([_squad([unit])], 1, [])

	assert_int(unit.wounded).is_equal(3)
