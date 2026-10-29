extends GdUnitTestSuite
## FormationSimulation, per specs/22-formation-feel-test.md and Decision 40: squads in
## formation on one lane - the front band fights and the rest come on when it falls. Units seek
## contact (Decision 88): the scrum is the only fight (spec 30, round 1 part 1).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const ROUTE := 9.0
const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(hp: int, dmg: int, speed: float, depth: int = 1, width: int = 1) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = speed
	unit_def.footprint_depth = depth
	unit_def.footprint_width = width
	return unit_def


func _grem() -> UnitDef:
	return _def(20, 6, 1.0)


func _militia() -> UnitDef:
	return _def(26, 5, 0.8)


## A single-rank line of `count` units, `width` wide.
func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _run(sim: FormationSimulation, done: Callable, limit: int = 4000) -> Array:
	var log := []
	for i in range(limit):
		log.append_array(sim.step())
		if done.call():
			break
	return log


func _of(events: Array, kind: String) -> Array:
	return events.filter(func(e): return e["type"] == kind)


func test_squads_meet_and_stop_at_melee_reach() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(1, _line(_grem(), 1), "player", true)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)

	_run(sim, func(): return mine.state == SkirmishSquad.State.FIGHTING)

	assert_int(theirs.state).is_equal(SkirmishSquad.State.FIGHTING)
	var gap := theirs.front_distance - mine.front_distance
	assert_float(gap).is_less_equal(FormationSimulation.MELEE_REACH + 0.0001)
	assert_float(gap).is_greater(0.0)


func test_the_spec_21_duel_still_holds() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(1, _line(_grem(), 1), "player", true)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)

	var log := _run(sim, func(): return mine.is_destroyed())

	var first: int = _of(log, "hit")[0]["tick"]  # from the first blow, as spec 21 counts
	assert_float((sim.tick_number() - first) * TICK).is_equal_approx(3.0, 0.0001)
	assert_int(UnitHealth.of_one(theirs, theirs.units[0])).is_equal(2)


func test_a_back_band_unit_fights_only_once_the_front_has_fallen() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var rear := _grem()
	rear.preferred_position = UnitDef.Position.BACK  # it keeps its place while a front one is left
	var column := [[_grem(), Vector2i(0, 0)], [rear, Vector2i(1, 0)]]
	var mine := sim.spawn_squad(1, column, "player", true)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)
	var front_id := mine.units[0].id
	var back_id := mine.units[1].id

	# The militia beats the front grem on 2 hp; the grem behind comes on and finishes it.
	var log := _run(sim, func(): return theirs.is_destroyed() or mine.is_destroyed())

	var downed := _of(log, "downed").filter(func(e): return e["unit"] == front_id)
	var first_death: int = downed[0]["tick"]
	var back_hits := _of(log, "hit").filter(func(e): return e["unit"] == back_id)
	assert_bool(back_hits.all(func(e): return e["tick"] > first_death)).is_true()
	assert_int(back_hits.size()).is_greater(0)


func test_a_unit_struck_by_several_takes_every_blow_but_strikes_once() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	sim.spawn_squad(5, _line(_grem(), 5), "player", true)
	var theirs := sim.spawn_squad(3, _line(_militia(), 3), "the_kingdom", false)

	var log := _run(sim, func(): return theirs.is_destroyed(), 600)

	for tick in range(1, sim.tick_number() + 1):
		var hits := _of(log, "hit").filter(func(e): return e["tick"] == tick)
		var strikers := hits.map(func(e): return e["unit"])
		for striker in strikers:
			assert_int(strikers.count(striker)).is_equal(1)  # one blow a striker a tick
	var crowded := false
	for target in theirs.units:
		var taken := _of(log, "hit").filter(func(e): return e["target"] == target.id)
		for i in range(1, taken.size()):  # every blow lands in full, however many strike
			var before: int = taken[i - 1]["target_hp"]
			assert_int(taken[i]["target_hp"]).is_equal(before - taken[i]["dmg"])
		var strikers := {}
		for hit in taken:
			strikers[hit["unit"]] = true
		crowded = crowded or strikers.size() > 1
	assert_bool(crowded).is_true()  # the wider line's ends close in: some are struck by two


func test_a_holding_squad_stays_put_and_still_fights() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(1, _line(_grem(), 1), "player", true)
	sim.order(mine.id, SkirmishUnit.Order.HOLD)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)

	_run(sim, func(): return theirs.state == SkirmishSquad.State.FIGHTING)

	assert_float(mine.front_distance).is_equal(0.0)
	assert_int(mine.state).is_equal(SkirmishSquad.State.FIGHTING)


func test_a_retreating_squad_disengages_and_an_enemy_that_won_t_pursue_holds() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(1, _line(_grem(), 1), "player", true)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)
	theirs.pursues = false  # ordered not to: it stays at its post (Decision 109)
	_run(sim, func(): return mine.state == SkirmishSquad.State.FIGHTING)
	var at := UnitPlaces.of(mine, mine.units[0]).x
	var post := theirs.front_distance

	sim.order(mine.id, SkirmishUnit.Order.RETREAT)
	var events := sim.step()

	assert_array(events.map(func(e): return e["type"])).contains(["disengaged", "withdrawing"])
	# It withdraws from where it stands (Decision 99), heading home; the enemy holds its
	# post, though its units may strike it while it is in reach (Decision 111).
	_run(sim, func(): return false, 20)
	assert_float(UnitPlaces.of(mine, mine.units[0]).x).is_less(at)
	assert_float(theirs.front_distance).is_equal(post)
	assert_bool(theirs.pursuit.is_empty()).is_true()


func test_reaching_the_enemy_end_is_arrival() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(2, _line(_grem(), 2), "player", true)

	var log := _run(sim, func(): return mine.state == SkirmishSquad.State.ARRIVED)

	assert_float(mine.front_distance).is_equal(ROUTE)
	assert_int(_of(log, "arrived").size()).is_equal(1)


func test_a_destroyed_squad_frees_its_opponent_to_move_on() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(1, _line(_grem(), 1), "player", true)
	var theirs := sim.spawn_squad(1, _line(_militia(), 1), "the_kingdom", false)
	_run(sim, func(): return mine.is_destroyed())

	assert_int(mine.state).is_equal(SkirmishSquad.State.DESTROYED)
	var at := theirs.front_distance
	_run(sim, func(): return theirs.front_distance < at, 100)  # it re-forms, then marches on
	assert_float(theirs.front_distance).is_less(at)


func test_the_same_inputs_replay_to_the_same_events() -> void:
	var runs := []
	for run in range(2):
		var sim := FormationSimulation.new(ROUTE, TICK)
		var mine := sim.spawn_squad(5, _line(_grem(), 5), "player", true)
		sim.spawn_squad(3, _line(_militia(), 3), "the_kingdom", false)
		var log := []
		for i in range(600):
			if i == 200:
				sim.order(mine.id, SkirmishUnit.Order.RETREAT)
			log.append_array(sim.step())
		runs.append(JSON.stringify(log))

	assert_str(runs[0]).is_equal(runs[1])


func test_the_crate_issues_the_ids_and_queues_the_orders() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var two := [[_grem(), Vector2i(0, 0)], [_grem(), Vector2i(0, 1)]]
	var first := sim.spawn_squad(2, two, "player", true)
	var second := sim.spawn_squad(1, [[_grem(), Vector2i(0, 0)]], "the_kingdom", false)
	assert_array([first.id, second.id]).is_equal([1, 2])
	var ids := (first.units + second.units).map(func(unit): return unit.id)
	assert_array(ids).is_equal([1, 2, 3])
	sim.order(second.id, SkirmishUnit.Order.HOLD)
	sim.order(first.id, SkirmishUnit.Order.HOLD)
	sim.order(second.id, SkirmishUnit.Order.RETREAT)
	var queued: PackedInt64Array = sim.store.field().orders_pending()
	assert_array(Array(queued)).is_equal(
		[2, SkirmishUnit.Order.RETREAT, 1, SkirmishUnit.Order.HOLD]
	)
	sim.step()
	assert_bool(sim.store.field().orders_pending().is_empty()).is_true()
	assert_int(first.order).is_equal(SkirmishUnit.Order.HOLD)
