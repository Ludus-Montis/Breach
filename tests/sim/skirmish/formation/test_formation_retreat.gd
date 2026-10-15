extends GdUnitTestSuite
## Retreat and pursuit, per Decisions 95 and 99 and spec 27 rounds 8 and 10: an ordered
## retreat breaks contact at once; a drilled formation withdraws fighting and gets away
## cheaply, a ragged one turns and runs, takes a scaled rout and pays in losses; an enemy
## ordered to pursue, or led by a pursuer, follows; one that isn't returns to formation,
## though its undisciplined units may break ranks to chase, decided unit by unit.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const ScrumPursuit = preload("res://sim/skirmish/formation/scrum_pursuit.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


func _def(discipline: int, tactics: Array[String] = []) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 60
	unit_def.items = [WeaponDef.innate_weapon(3)]
	unit_def.speed = 1.0
	unit_def.discipline = discipline
	unit_def.tactics = tactics
	return unit_def


func _row(unit_def: UnitDef) -> Array:
	var placements := []
	for column in range(8):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


## A head-on fight of 8 v 8; 3 s in, the player's squad is ordered to retreat. Returns
## [sim, mine, theirs, log of its withdrawal, the player's losses in it, the morale it lost
## on the order's tick, the damage it took in it]. `pursue` false orders the enemy not to.
func _retreat(mine: UnitDef, theirs: UnitDef, battle_seed: int = 1, pursue := true) -> Array:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = battle_seed
	sim.blow_rolls = true
	var me := sim.spawn_squad(8, _row(mine), "player", true)
	var foe := sim.spawn_squad(8, _row(theirs), "the_kingdom", false)
	foe.pursues = pursue
	for _i in range(400):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	for _i in range(30):
		sim.step()
	sim.order(me.id, SkirmishUnit.Order.RETREAT)
	var before := me.morale
	var log := sim.step()
	var shock := before - me.morale
	for _i in range(400):  # until it is safe and re-forms (Decision 99)
		log.append_array(sim.step())
		if me.withdraw.is_empty():
			break
	var lost := log.filter(func(e): return e["type"] == "died" and e["faction"] == "player")
	var hits := log.filter(func(e): return e["type"] == "hit" and e["faction"] == "the_kingdom")
	var taken: int = hits.reduce(func(total, e): return total + e["dmg"], 0)
	return [sim, me, foe, log, lost.size(), shock, taken]


func test_a_retreat_breaks_contact_at_once_and_heads_home() -> void:
	var setup := _retreat(_def(60), _def(60))
	var me: SkirmishSquad = setup[1]
	var log: Array = setup[3]

	assert_bool(log.any(func(e): return e["type"] == "disengaged")).is_true()
	assert_float(me.position.x).is_less(60.0)


func test_a_drilled_withdrawal_costs_less_than_a_ragged_flight() -> void:
	var drilled := 0
	var ragged := 0
	for battle_seed in range(1, 6):  # from an enemy pursuing as a body (Decision 101)
		drilled += _retreat(_def(60), _def(60), battle_seed, true)[6]
		ragged += _retreat(_def(20), _def(60), battle_seed, true)[6]

	assert_int(drilled).is_less(ragged)


func test_a_ragged_retreat_takes_a_scaled_rout() -> void:
	var ragged := _retreat(_def(20), _def(60))
	var drilled := _retreat(_def(60), _def(60))

	assert_int(ragged[5]).is_greater_equal(BattleTuning.current().pursuit_ragged_shock * 3 / 5 - 2)
	assert_int(drilled[5]).is_less(ragged[5])


func test_an_enemy_pursues_by_default_unless_ordered_not_to() -> void:
	var pursued := _retreat(_def(60), _def(60), 1, true)
	var held := _retreat(_def(60), _def(60), 1, false)

	assert_bool(pursued[3].any(func(e): return e["type"] == "pursuing")).is_true()
	assert_bool(held[3].any(func(e): return e["type"] == "pursuing")).is_false()


func test_undisciplined_units_break_ranks_to_chase_one_by_one() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var me := sim.spawn_squad(8, _row(_def(60)), "player", true)
	var ragged := sim.spawn_squad(8, _row(_def(0)), "the_kingdom", false)
	ragged.pursues = false  # ordered not to: its units break ranks all the same
	for _i in range(400):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	for _i in range(30):
		sim.step()
	sim.order(me.id, SkirmishUnit.Order.RETREAT)
	var most := 0
	for _i in range(5):
		sim.step()
		most = maxi(most, ragged.chasers.size())

	assert_int(most).is_greater(0)
	assert_int(most).is_less(ragged.living().size())  # some chase, not all


func test_a_pursuing_formation_follows_as_a_body_then_returns_to_its_post() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 32), Vector2(128, 32)]))
	var me := sim.spawn_squad(8, _row(_def(20)), "player", true, 0, route)
	var line := sim.spawn_squad(8, _row(_def(60)), "the_kingdom", false, 0, route)
	line.front_distance = 80.0 / 64.0
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	line.pursues = true
	for _i in range(400):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	for _i in range(20):
		sim.step()
	sim.order(me.id, SkirmishUnit.Order.RETREAT)
	var furthest := line.position.x
	var log := []
	for _i in range(900):
		log.append_array(sim.step())
		furthest = minf(furthest, line.position.x)

	assert_float(furthest).is_less(70.0)  # the formation itself followed
	assert_bool(log.any(func(e): return e["type"] == "pursuit_ended")).is_true()
	assert_float(line.position.x).is_greater(76.0)  # and went back to its post
	assert_int(line.order).is_equal(SkirmishUnit.Order.HOLD)
	assert_bool(line.pursuit.is_empty()).is_true()


func test_a_unit_breaking_ranks_chases_no_further_than_its_own_leash() -> void:
	var chased := 0
	var furthest := 0.0
	for battle_seed in range(1, 11):  # until some units break ranks
		var sim := FormationSimulation.new(2.0, 0.1)
		sim.fight_seed = battle_seed
		var me := sim.spawn_squad(8, _row(_def(10)), "player", true)  # ragged: it flees far
		var keen := sim.spawn_squad(8, _row(_def(30)), "the_kingdom", false)  # 128 cells
		keen.pursues = false
		for _i in range(400):
			if sim.step().any(func(e): return e["type"] == "engaged"):
				break
		for _i in range(30):
			sim.step()
		sim.order(me.id, SkirmishUnit.Order.RETREAT)
		for _i in range(600):
			sim.step()
			chased += keen.chasers.size()
			for unit_id in keen.chasers:
				var at: Vector2 = SquadEntries.loose_at(keen, keen.chasers[unit_id]["unit"])
				furthest = maxf(furthest, at.distance_to(keen.chasers[unit_id]["from"]))
		assert_bool(keen.chasers.is_empty()).is_true()  # all came back
		if chased > 0:
			break

	assert_int(chased).is_greater(0)
	assert_float(furthest).is_less_equal(128.0 + 1.0)


func _caught_at_home(captain: String) -> Array:
	var log := (
		"seed 233831 captain %s\n0 pursues off\n115 send A\n186 pursues on\n223 retreat A" % captain
	)
	var events := []
	var field := FormationFieldActions.replay(
		log, 480, func(_f, tick_events): events.append_array(tick_events)
	)
	var wave: SkirmishSquad = field.sim.squads().filter(func(s): return s.faction_id == "player")[0]
	return [field, wave, events]


func test_a_wave_caught_at_home_by_units_breaking_ranks_fights_back() -> void:
	# A feel-test log: the line's chasers ran A's last three down at its spawn and A never
	# struck back (Decision 111). Without a captain, the militia break ranks to chase.
	var setup := _caught_at_home("off")
	var wave: SkirmishSquad = setup[1]
	var events: Array = setup[2]

	var homes := events.filter(func(e): return e["type"] == "returned" and e["squad"] == wave.id)
	var home: int = homes[0]["tick"]
	var fought_back := events.any(
		func(e): return e["type"] == "engaged" and e["squad"] == wave.id and e["tick"] > home
	)
	assert_bool(fought_back).is_true()
	var struck := events.filter(
		func(e): return e["type"] == "hit" and e["faction"] == "player" and e["tick"] > home
	)
	assert_int(struck.size()).is_greater(0)  # it strikes the chasers back


func test_a_captain_holds_its_units_from_breaking_ranks() -> void:
	# The same log, the line led: its militia, steadied by their captain, don't run on past
	# their formation's leash (Decision 112), and A's survivors get home alive.
	var setup := _caught_at_home("on")

	assert_int(setup[1].living().size()).is_greater(0)
	assert_int(setup[0].kingdom_line.living().size()).is_equal(12)


func test_a_formation_marches_home_without_waiting_for_its_runaways() -> void:
	var log := "seed 233831 captain off\n0 pursues off\n115 send A\n186 pursues on\n223 retreat A"
	var seen := {"last": null, "moved": 0.0}  # a lambda captures a local by value
	FormationFieldActions.replay(
		log,
		480,
		func(field, _events):
			var line: SkirmishSquad = field.kingdom_line
			var homeward: bool = not line.pursuit.is_empty() and line.pursuit["returning"]
			if homeward and not line.chasers.is_empty():
				if seen["last"] != null:
					seen["moved"] += absf(line.front_distance - seen["last"])
				seen["last"] = line.front_distance
			else:
				seen["last"] = null
	)

	assert_float(seen["moved"]).is_greater(0.0)  # its runaways make their own way back
