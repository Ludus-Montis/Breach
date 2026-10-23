extends GdUnitTestSuite
## A retreat that is already home (FormationMarch.check_ends): a group ordered to retreat
## whose units all stand home, within HOME_CELLS, arrives and holds as it faces - no
## about-face that carries its front a rank past home, and another back, for ever - and
## then forms up with its command's other group there (FormationGroups).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const GREM := "grem"


func _group(sim: FormationSimulation, count: int, front_cells: float) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit(GREM)
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, "player", true)
	squad.front_distance = front_cells / 64.0
	FormationMarch.sync_units([squad])  # placed there
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	return squad


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _tick in ticks:
		log.append_array(sim.step())
	return log


func _of(log: Array, kind: String, squad: SkirmishSquad) -> Array:
	return log.filter(func(e): return e["type"] == kind and e["squad"] == squad.id)


func test_a_group_at_home_ordered_to_retreat_holds_without_turning() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var group := _group(sim, 2, 0.0)
	_run(sim, 5)
	sim.order(group.id, SkirmishUnit.Order.RETREAT)
	var log := _run(sim, 120)
	assert_array(_of(log, "turning", group)).is_empty()
	assert_int(_of(log, "returned", group).size()).is_equal(1)
	assert_int(group.order).is_equal(SkirmishUnit.Order.HOLD)
	assert_int(group.state).is_equal(SkirmishSquad.State.HOLDING)
	assert_float(group.front_distance).is_equal(group.home_distance)


func test_a_retreat_whose_units_are_home_arrives_though_its_front_is_not() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var group := _group(sim, 2, 0.8)  # its front 0.8 cells out: its rank within HOME_CELLS
	_run(sim, 5)
	var facing := group.heading
	sim.order(group.id, SkirmishUnit.Order.RETREAT)
	var log := _run(sim, 120)
	assert_array(_of(log, "turning", group)).is_empty()
	assert_int(group.order).is_equal(SkirmishUnit.Order.HOLD)
	assert_float(group.heading).is_equal(facing)  # it holds as it faces


func test_a_retreat_still_out_marches_home_and_returns() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var group := _group(sim, 2, 12.0)
	_run(sim, 5)
	sim.order(group.id, SkirmishUnit.Order.RETREAT)
	var log := _run(sim, 200)
	assert_int(_of(log, "turning", group).size()).is_greater(0)  # it turns for home
	assert_int(_of(log, "returned", group).size()).is_equal(1)
	assert_int(group.order).is_equal(SkirmishUnit.Order.HOLD)


func test_home_it_forms_up_with_another_group_standing_there() -> void:
	# Two groups back from a fight, standing together: by FormationGroups' rules they form
	# up (leaderless, one set to merge) once the retreat has arrived and holds.
	var sim := FormationSimulation.new(4.0, 0.1)
	var first := _group(sim, 2, 0.8)
	var second := _group(sim, 4, 0.0)
	first.merges = true
	for group in [first, second]:
		group.fought = true
	sim.order(first.id, SkirmishUnit.Order.RETREAT)
	var log := _run(sim, 60)
	assert_array(_of(log, "turning", first)).is_empty()
	assert_int(log.filter(func(e): return e["type"] == "merged").size()).is_equal(1)
	assert_int(sim.squads().size()).is_equal(1)
	assert_int(sim.squads()[0].living().size()).is_equal(6)
