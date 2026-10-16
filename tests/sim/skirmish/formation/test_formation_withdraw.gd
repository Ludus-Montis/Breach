extends GdUnitTestSuite
## Withdrawals and disorderly flight, per Decision 99 and spec 27 round 10: a retreat is
## combat's equal; its units flee homeward from where they stand, not via their places;
## it re-forms on its route only once safe, then marches home; one with nowhere further to
## go re-forms at home; the less ordered it is the wider it fans out, and a rout fans out
## wholly unless a friend stands between it and home.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationManoeuvre = preload("res://sim/skirmish/formation/formation_manoeuvre.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def(discipline: int) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 60
	unit_def.items = [WeaponDef.innate_weapon(3)]
	unit_def.speed = 1.0
	unit_def.discipline = discipline
	return unit_def


func _row(unit_def: UnitDef, count: int = 8) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


## 8 v 8 head-on; 3 s into the fight the player's squad is ordered to retreat.
## Returns [sim, mine, theirs].
func _fight_then_retreat(discipline: int) -> Array:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = 3
	var me := sim.spawn_squad(8, _row(_def(discipline)), "player", true)
	var foe := sim.spawn_squad(8, _row(_def(60)), "the_kingdom", false)
	for _i in range(400):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	for _i in range(30):
		sim.step()
	sim.order(me.id, SkirmishUnit.Order.RETREAT)
	return [sim, me, foe]


func _spread_y(squad: SkirmishSquad) -> float:
	var ys: Array = squad.living().map(func(u): return ScrumReach.at(squad, u).y)
	return ys.max() - ys.min()


func test_a_retreat_withdraws_at_combats_priority_from_where_its_units_stand() -> void:
	var setup := _fight_then_retreat(60)
	var me: SkirmishSquad = setup[1]
	var foe: SkirmishSquad = setup[2]
	setup[0].step()
	var front: float = foe.living().map(func(u): return UnitPlaces.of(foe, u).x).min()
	var before := {}
	for unit in me.living():
		before[unit.id] = ScrumReach.at(me, unit).x

	for _i in range(10):
		setup[0].step()

	assert_int(me.manoeuvre).is_equal(FormationManoeuvre.Kind.WITHDRAW)
	for unit in me.living():  # every unit heads away, none back towards its place first
		assert_float(ScrumReach.at(me, unit).x).is_less(before[unit.id])
		assert_float(ScrumReach.at(me, unit).x).is_less(front)


func test_it_re_forms_only_once_safe_then_marches_home() -> void:
	var setup := _fight_then_retreat(60)
	var me: SkirmishSquad = setup[1]
	setup[0].order(setup[2].id, SkirmishUnit.Order.HOLD)  # the enemy holds its ground
	var log := []
	for _i in range(600):
		log.append_array(setup[0].step())

	var regrouped: Array = log.filter(func(e): return e["type"] == "regrouping")
	var withdrew: Array = log.filter(func(e): return e["type"] == "withdrawing")
	assert_bool(regrouped.is_empty()).is_false()
	var safe_ticks := roundi(BattleTuning.current().rout_rally_seconds / 0.1)
	assert_int(regrouped[0]["tick"] - withdrew[0]["tick"]).is_greater_equal(safe_ticks)
	assert_bool(me.withdraw.is_empty()).is_true()
	assert_bool(log.any(func(e): return e["type"] == "returned" and e["squad"] == me.id)).is_true()


func test_a_ragged_retreat_fans_out_wider_than_a_drilled_one() -> void:
	var drilled := _fight_then_retreat(60)
	var ragged := _fight_then_retreat(0)
	for _i in range(30):
		drilled[0].step()
		ragged[0].step()

	assert_float(_spread_y(ragged[1])).is_greater(_spread_y(drilled[1]) + 1.0)


func test_a_rout_with_no_friend_in_its_way_fans_out_from_its_route() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = 5
	var mine := sim.spawn_squad(4, _row(_def(30), 4), "player", true)
	mine.front_distance = 60.0 / 64.0  # far enough from home that its run doesn't end
	sim.step()
	var start := _spread_y(mine)
	mine.morale = 0

	for _i in range(40):
		sim.step()

	assert_int(mine.state).is_equal(SkirmishSquad.State.ROUTING)
	var ys: Array = mine.living().map(func(u): return FormationRout.where(mine, u.id).y)
	assert_float(ys.max() - ys.min()).is_greater(start + 2.0)


func test_withdrawals_replay_the_same() -> void:
	var logs := []
	for _r in range(2):
		var setup := _fight_then_retreat(10)
		var log := []
		for _i in range(200):
			log.append_array(setup[0].step())
		logs.append(log)

	assert_array(logs[1]).is_equal(logs[0])


## Both sides of a ragged head-on fight ordered to retreat together, stepped with every
## squad and unit list reversed each tick if `reversed`. Returns each tick's state.
func _both_retreat(reversed: bool) -> Array:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = 7
	var me := sim.spawn_squad(8, _row(_def(10)), "player", true)
	var foe := sim.spawn_squad(8, _row(_def(10)), "the_kingdom", false)
	var states := []
	for tick in range(260):
		if tick == 120:
			sim.order(me.id, SkirmishUnit.Order.RETREAT)
			sim.order(foe.id, SkirmishUnit.Order.RETREAT)
		if reversed:
			sim.squads().reverse()
			for squad in sim.squads():
				squad.units.reverse()
		sim.step()
		var state := []
		for squad in [me, foe]:
			for unit in squad.living():
				state.append(
					[
						unit.id,
						UnitHealth.of(squad, unit),
						ScrumReach.at(squad, unit).snapped(Vector2.ONE * 0.0001)
					]
				)
		state.sort()
		states.append(state)
	return states


func test_withdrawals_do_not_hang_on_list_order() -> void:
	assert_array(_both_retreat(true)).is_equal(_both_retreat(false))


## The feel test's field (captained line, seed 606531, from the user's test of #90): A is
## sent and, 3 s into its fight, ordered home with the line set to pursue. Returns [field,
## A, the log from the order on].
func _a_retreats_from_a_pursuing_line(pursues := true) -> Array:
	var field := FormationField.new(
		0.1,
		load("res://content/units/grem.tres"),
		8,
		load("res://content/units/kingdom_militia.tres"),
		load("res://content/units/grem_chieftain.tres"),
		load("res://content/units/kingdom_captain.tres"),
		606531
	)
	for _i in range(2000):
		if field.waves["A"].built() == 8:
			break
		field.step()
	field.kingdom_line.pursues = pursues  # it does by default (Decision 109)
	var wave := field.send("A")
	for _i in range(400):
		if field.step().any(func(e): return e["type"] == "engaged"):
			break
	for _i in range(30):
		field.step()
	field.sim.order(wave.id, SkirmishUnit.Order.RETREAT)
	return [field, wave, []]


func test_a_pursuit_moves_as_a_body_and_gives_up_at_its_leash() -> void:
	var setup := _a_retreats_from_a_pursuing_line()
	var line: SkirmishSquad = setup[0].kingdom_line
	var leash := FormationDiscipline.pursuit_leash(line)  # a captained line: 64 cells
	var post := line.front_distance
	var gone := 0.0
	var spread := 0.0
	var log := []
	for _i in range(400):
		log.append_array(setup[0].step())
		var xs: Array = line.living().map(func(u): return ScrumReach.at(line, u).x)
		var captains := line.living().filter(func(u): return u.leadership > 0)
		if not line.pursuit.is_empty() and not line.pursuit["returning"] and captains.size() > 0:
			spread = maxf(spread, xs.max() - ScrumReach.at(line, captains[0]).x)  # it runs west
		gone = maxf(gone, absf(line.front_distance - post) * MapLayoutDef.CELLS_PER_TILE)

	assert_float(leash).is_equal(64.0)
	assert_float(spread).is_less(6.0)  # its captain no longer runs ahead of its rearmost
	assert_bool(log.any(func(e): return e["type"] == "pursuit_ended")).is_true()
	assert_float(gone).is_less_equal(leash + 1.0)  # not all the way to A's spawn


func test_a_pursuit_ends_when_the_enemy_is_out_of_sight() -> void:
	var setup := _a_retreats_from_a_pursuing_line()
	var line: SkirmishSquad = setup[0].kingdom_line
	for unit in line.units:
		unit.detection = 1.0  # it loses sight of A's wave as soon as it steps back
	var ended := -1
	for _i in range(100):
		for event in setup[0].step():
			if event["type"] == "pursuit_ended" and ended < 0:
				ended = setup[0].sim.tick_number()

	assert_int(ended).is_greater(0)


func test_a_withdrawal_home_holds_there_facing_out() -> void:
	var setup := _a_retreats_from_a_pursuing_line(false)  # so that it gets home
	var wave: SkirmishSquad = setup[1]
	var lateral := 0.0
	var log := []
	for _i in range(400):
		log.append_array(setup[0].step())
		for unit in wave.living():
			lateral = maxf(lateral, absf(ScrumReach.at(wave, unit).y - 32.0))

	var mine: Array = log.filter(func(e): return e.get("squad") == wave.id)
	var home: int = mine.filter(func(e): return e["type"] == "returned")[0]["tick"]
	var turns: Array = mine.filter(func(e): return e["type"] == "turning" and e["tick"] > home)
	assert_int(turns.size()).is_equal(0)  # it doesn't spin at its spawn
	assert_int(wave.order).is_equal(SkirmishUnit.Order.HOLD)
	var fan := BattleTuning.current().rout_fan_cells
	assert_float(lateral).is_less_equal(fan + 4.0)  # route A's line is y 32
