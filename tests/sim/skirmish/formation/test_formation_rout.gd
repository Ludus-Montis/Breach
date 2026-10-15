extends GdUnitTestSuite
## Rout, per Decision 82 and spec 27 round 3: at 0 morale a formation breaks; its units
## flee home along its route, strike nothing and are struck from behind; they crush
## friends they run through; they rally at a led formation, or re-form round their own
## leader when no enemy is near; and those reaching home leave the field.

const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(hp: int, dmg: int, speed: float = 1.0, leadership: int = 0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = speed
	unit_def.leadership = leadership
	return unit_def


func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(sim.step())
	return log


func _of(log: Array, kind: String) -> Array:
	return log.filter(func(e): return e["type"] == kind)


## The player's squad 30 cells from home, fighting a kingdom squad; returns [sim, mine, theirs].
func _fight(their_speed: float = 1.0, mine_def: UnitDef = null) -> Array:
	var sim := FormationSimulation.new(2.0, TICK)
	var mine := sim.spawn_squad(4, _line(mine_def if mine_def else _def(100, 2), 4), "player", true)
	mine.front_distance = 30.0 / 64.0
	var theirs := sim.spawn_squad(4, _line(_def(500, 2, their_speed), 4), "the_kingdom", false)
	theirs.front_distance = 32.0 / 64.0
	FormationMarch.sync_units(sim.squads())  # placed there
	_run(sim, 5)
	return [sim, mine, theirs]


func test_a_formation_at_no_morale_routs_and_its_fight_ends() -> void:
	var setup := _fight()
	var mine: SkirmishSquad = setup[1]
	assert_int(mine.state).is_equal(SkirmishSquad.State.FIGHTING)
	mine.morale = 0

	var log := _run(setup[0], 1)

	assert_int(_of(log, "routed").size()).is_equal(1)
	assert_int(mine.state).is_equal(SkirmishSquad.State.ROUTING)
	assert_int(setup[2].engaged_with).is_equal(0)


func test_routers_flee_home_and_strike_nothing() -> void:
	var setup := _fight()
	var mine: SkirmishSquad = setup[1]
	mine.morale = 0
	_run(setup[0], 1)
	var before: float = UnitPlaces.of(mine, mine.living()[0]).x

	var log := _run(setup[0], 10)

	assert_float(UnitPlaces.of(mine, mine.living()[0]).x).is_less(before - 5.0)
	assert_int(_of(log, "hit").filter(func(h): return h["faction"] == "player").size()).is_equal(0)


func test_hostiles_strike_routers_running_past_them_from_behind() -> void:
	var setup := _fight()
	var sim: FormationSimulation = setup[0]
	var mine: SkirmishSquad = setup[1]
	var across := sim.spawn_squad(4, _line(_def(500, 2), 4), "the_kingdom", false)
	across.front_distance = 15.0 / 64.0  # a hostile line between the routers and home
	FormationMarch.sync_units([across])  # placed there
	sim.order(across.id, SkirmishUnit.Order.HOLD)
	mine.morale = 0
	var ids := mine.units.map(func(u): return u.id)  # before any reach home and leave

	var log := _run(sim, 40)

	var broke: int = _of(log, "routed")[0]["tick"]
	var on_routers := _of(log, "hit").filter(
		func(h): return ids.has(h["target"]) and h["tick"] > broke
	)
	assert_bool(on_routers.is_empty()).is_false()
	assert_bool(on_routers.all(func(h): return h["flank"])).is_true()


func test_routers_crush_friends_they_run_through() -> void:
	var setup := _fight()
	var sim: FormationSimulation = setup[0]
	var mine: SkirmishSquad = setup[1]
	var behind := sim.spawn_squad(4, _line(_def(100, 2), 4), "player", true)
	behind.front_distance = 15.0 / 64.0
	sim.order(behind.id, SkirmishUnit.Order.HOLD)
	_run(sim, 1)
	var calm := behind.morale
	mine.morale = 0

	var log := _run(sim, 20)

	assert_bool(_of(log, "crushed").is_empty()).is_false()
	assert_int(behind.morale).is_less(calm)
	assert_bool(behind.living().any(func(u): return u.hp < 100)).is_true()


func test_routers_rally_to_a_led_formation() -> void:
	var setup := _fight()
	var sim: FormationSimulation = setup[0]
	var mine: SkirmishSquad = setup[1]
	var led := sim.spawn_squad(2, _line(_def(100, 2, 1.0, 2), 2), "player", true)
	led.front_distance = 15.0 / 64.0
	sim.order(led.id, SkirmishUnit.Order.HOLD)
	_run(sim, 1)
	mine.morale = 0

	var log := _run(sim, 30)

	assert_bool(_of(log, "rallied").is_empty()).is_false()
	assert_int(led.living().size()).is_greater(2)


func test_a_rout_re_forms_round_its_own_leader_with_no_enemy_near() -> void:
	var placements := _line(_def(100, 2), 3)
	placements.append([_def(100, 2, 0.5, 2), Vector2i(0, 3)])  # a slow leader
	var sim := FormationSimulation.new(4.0, TICK)
	var mine := sim.spawn_squad(4, placements, "player", true)
	mine.front_distance = 2.0
	_run(sim, 1)
	mine.morale = 0

	var log := _run(sim, 80)

	assert_int(_of(log, "reformed").size()).is_equal(1)
	assert_int(mine.state).is_equal(SkirmishSquad.State.HOLDING)
	assert_int(mine.morale).is_greater(0)


func test_routers_reaching_home_leave_the_field() -> void:
	var setup := _fight()
	var mine: SkirmishSquad = setup[1]
	mine.morale = 0
	for unit in mine.units:
		unit.courage = 100  # none surrenders when struck (FormationWounds)

	var log := _run(setup[0], 60)

	var fled := _of(log, "fled_home")
	assert_int(fled.size()).is_equal(4)
	assert_object(fled[0]["definition"]).is_not_null()
	assert_int(mine.state).is_equal(SkirmishSquad.State.DESTROYED)


func test_a_rout_replays_the_same() -> void:
	var logs := []
	for _r in range(2):
		var setup := _fight(1.2)
		setup[1].morale = 0
		logs.append(_run(setup[0], 40).map(func(e): return [e["type"], e["tick"]]))

	assert_array(logs[1]).is_equal(logs[0])


func test_a_rout_re_forms_facing_its_route_s_true_heading() -> void:
	# Spec 30 round 2's tidy: it re-formed facing the nearest of four ways, not a slanting
	# route square on.
	var FormationRoute = load("res://sim/skirmish/formation/formation_route.gd")
	var FormationRout = load("res://sim/skirmish/formation/formation_rout.gd")
	var route = FormationRoute.new(PackedVector2Array([Vector2(0, 0), Vector2(30, 40)]))
	var sim := FormationSimulation.new(10.0, TICK)
	var squad := sim.spawn_squad(2, _line(_def(10, 1, 1.0, 1), 2), "player", true, 0, route)
	for unit in squad.living():
		squad.fleeing[unit.id] = {"along": 10.0, "offset": Vector2.ZERO}

	FormationRout._reform(squad, 0, [], 1)

	assert_float(squad.heading).is_equal_approx(rad_to_deg(atan2(30, -40)) + 0.0, 0.01)
