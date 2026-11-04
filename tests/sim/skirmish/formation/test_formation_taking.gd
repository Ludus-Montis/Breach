extends GdUnitTestSuite
## FormationTaking (spec 30 round 3, part 8): a group out of a fight walks free units out
## to the downed foes it sees within take_reach, one to a body, and holds while they take
## them; they come back when done or when the hold runs out. "Fall behind, left behind"
## and a command that leaves the downed take none.

const FormationTaking = preload("res://sim/skirmish/formation/formation_taking.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationPerception = preload("res://sim/skirmish/formation/formation_perception.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## A holding player line `count` wide, its front 40 cells along, and a downed kingdom unit
## `gap` cells ahead of its front. Returns [sim, line, body].
func _setup(count: int, gap: float) -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var line := sim.spawn_squad(count, placements, "player", true)
	line.front_distance = 40.0 / 64.0
	line.order = SkirmishUnit.Order.HOLD
	FormationMarch.sync_units([line])
	var foe := sim.spawn_squad(1, [[grem, Vector2i(0, 0)]], "the_kingdom", false)
	sim.order(foe.id, SkirmishUnit.Order.HOLD)
	var body: SkirmishUnit = foe.units[0]
	UnitPlaces.put(foe, body, Vector2(40.0 + gap, line.position.y))
	UnitHealth.put_one(foe, body, 0)
	UnitState.put(foe, body, SkirmishUnit.State.DOWNED)
	foe.store.columns.put(body.slot, UnitColumns.WAKE_LEFT, 999.0)
	return [sim, line, body]


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in ticks:
		log.append_array(sim.step())
	return log


func test_a_free_unit_walks_out_and_takes_a_downed_foe_while_its_group_holds() -> void:
	var setup := _setup(3, 4.0)
	var line: SkirmishSquad = setup[1]
	var body: SkirmishUnit = setup[2]

	var log := _run(setup[0], 1)
	assert_bool(log.any(func(e): return e["type"] == "sent_to_take")).is_true()
	assert_bool(FormationTaking.holding(line)).is_true()
	assert_float(FormationWalk.share(line)).is_equal(0.0)

	_run(setup[0], 120)
	assert_int(body.state).is_not_equal(SkirmishUnit.State.DOWNED)  # finished or taken
	assert_bool(FormationTaking.holding(line)).is_false()


func test_a_body_beyond_reach_is_left() -> void:
	var setup := _setup(3, BattleTuning.current().take_reach + 6.0)

	var log := _run(setup[0], 5)

	assert_bool(log.any(func(e): return e["type"] == "sent_to_take")).is_false()


func test_fall_behind_left_behind_presses_on_and_takes_none() -> void:
	var setup := _setup(2, 4.0)
	var line: SkirmishSquad = setup[1]
	for unit in line.units:
		unit.set_trait("fall_behind_left_behind", 1)

	var log := _run(setup[0], 5)

	assert_bool(log.any(func(e): return e["type"] == "sent_to_take")).is_false()


func test_a_command_that_leaves_the_downed_takes_none() -> void:
	var setup := _setup(2, 4.0)
	var line: SkirmishSquad = setup[1]
	line.command.takes = false

	var log := _run(setup[0], 5)

	assert_bool(log.any(func(e): return e["type"] == "sent_to_take")).is_false()


func test_the_hold_runs_out_and_the_group_marches_on() -> void:
	var setup := _setup(2, 4.0)
	var sim: FormationSimulation = setup[0]
	var line: SkirmishSquad = setup[1]
	var body: SkirmishUnit = setup[2]
	body.set_trait("regenerates_downed", 0)
	for unit in line.units:
		unit.dmg = 0  # it can't finish the body

	var hold := BattleTuning.current().take_hold
	_run(sim, roundi(hold / 0.1) + 5)

	assert_int(body.state).is_equal(SkirmishUnit.State.DOWNED)
	assert_bool(FormationTaking.holding(line)).is_false()
	assert_int(line.took_until).is_greater_equal(0)


func test_each_unit_takes_one_body_whatever_the_list_order() -> void:
	var picks := []
	for reverse in [false, true]:
		var setup := _setup(3, 3.0)
		var sim: FormationSimulation = setup[0]
		var line: SkirmishSquad = setup[1]
		var other: Array = _setup(1, 5.0)  # a second body, in another sim: move it over
		var second: SkirmishUnit = other[2]
		var lay := UnitPlaces.of(other[0].squads()[1], second)
		var foe: SkirmishSquad = sim.squads()[1]
		second.squad_id = foe.id
		second.faction_id = "the_kingdom"
		second.id = 999
		second.slot = sim.store.enrol(second.id)
		UnitState.admitted(sim.store, second)
		UnitPlaces.put(foe, second, lay)
		foe.units.append(second)
		UnitState.regather(foe)
		if reverse:
			sim.squads().reverse()
			line.units.reverse()
			UnitState.regather(line)
		_run(sim, 1)
		var taken := {}
		var taking := line.taking  # taker's id -> its body's store slot
		for unit_id in taking:
			taken[unit_id] = line.store.columns.value(taking[unit_id], UnitColumns.POSITION)
		picks.append(taken)
	assert_dict(picks[0]).is_equal(picks[1])
	assert_int(picks[0].size()).is_equal(2)


func test_the_crate_holds_who_is_out_taking_which_body() -> void:
	var setup := _setup(1, 3.0)
	var sim: FormationSimulation = setup[0]
	var line: SkirmishSquad = setup[1]
	var body: SkirmishUnit = setup[2]
	_run(sim, 1)

	var taking := line.taking
	assert_int(taking.size()).is_equal(1)
	assert_int(taking.values()[0]).is_equal(body.slot)
	assert_bool(line.units.any(func(u): return u.id == taking.keys()[0])).is_true()
	line.taking = {}
	assert_bool(FormationTaking.holding(line)).is_false()


## The taking is the crate's one pass (`taking/pass.rs`, spec 39 step 8b5-xxiii, whose cargo
## tests hold the rules): a threatened group sends none; else its taker sets out, held in the
## crate, and "sent_to_take" comes back as FormationEvents made it.
func test_the_crate_pass_sends_the_taker_and_tells_of_it() -> void:
	var setup := _setup(1, 3.0)
	var sim: FormationSimulation = setup[0]
	var line: SkirmishSquad = setup[1]
	var body: SkirmishUnit = setup[2]
	CrateEvents.taken(ColumnsLent.field(line.store.columns))  # the setup's own, dropped
	var events := []

	line.threatened = true
	FormationTaking.step(sim.squads(), 7, 0.1, sim.fight_seed, events)
	assert_array(events).is_empty()
	line.threatened = false
	FormationTaking.step(sim.squads(), 8, 0.1, sim.fight_seed, events)

	var taker: int = line.units[0].id
	assert_dict(line.taking).is_equal({taker: body.slot})
	assert_int(line.taking_since).is_equal(8)
	var told := {"type": "sent_to_take", "tick": 8, "squad": line.id, "faction": "player"}
	told["unit"] = taker
	assert_array(events).is_equal([told])


## The threat is the one the crate's perception kept (spec 39 step 8b5-xxxvii): a group that
## saw a standing foe as it looked sends no taker, with no perception handed to the taking.
func test_a_group_the_crate_saw_threatened_sends_none() -> void:
	var setup := _setup(1, 3.0)
	var sim: FormationSimulation = setup[0]
	var line: SkirmishSquad = setup[1]
	var grem: UnitDef = CrateContent.unit("grem")
	var foe := sim.spawn_squad(1, [[grem, Vector2i(0, 0)]], "the_kingdom", false)
	sim.order(foe.id, SkirmishUnit.Order.HOLD)
	UnitPlaces.put(foe, foe.units[0], Vector2(46.0, line.position.y))
	CrateEvents.taken(ColumnsLent.field(line.store.columns))  # the setup's own, dropped
	FormationPerception.see(sim.squads())
	var events := []

	FormationTaking.step(sim.squads(), 7, 0.1, sim.fight_seed, events)

	assert_array([line.taking, events]).is_equal([{}, []])
