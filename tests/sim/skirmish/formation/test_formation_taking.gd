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
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## A holding player line `count` wide, its front 40 cells along, and a downed kingdom unit
## `gap` cells ahead of its front. Returns [sim, line, body].
func _setup(count: int, gap: float) -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var grem: UnitDef = load("res://content/units/grem.tres")
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
	body.state = SkirmishUnit.State.DOWNED
	body.wake_left = 999.0
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
		unit.traits["fall_behind_left_behind"] = 1

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
	body.traits["regenerates_downed"] = 0
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
		UnitPlaces.put(foe, second, lay)
		foe.units.append(second)
		if reverse:
			sim.squads().reverse()
			line.units.reverse()
		_run(sim, 1)
		var taken := {}
		for unit_id in line.taking:
			taken[unit_id] = UnitPlaces.of(line, line.taking[unit_id])
		picks.append(taken)
	assert_dict(picks[0]).is_equal(picks[1])
	assert_int(picks[0].size()).is_equal(2)
