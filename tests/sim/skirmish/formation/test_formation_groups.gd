extends GdUnitTestSuite
## FormationGroups and GroupSplit (spec 30 round 3, part 4): units fallen behind go on as a
## group of the same command; groups meeting form up by one rule - one command's groups
## become one, a leader takes in the leaderless, two leaderless groups merge only if set
## to, and two led groups keep their own while units return to the command they started
## under. Decided by place and seeded draws, never by lists.

const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const GroupSplit = preload("res://sim/skirmish/formation/group_split.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## A player line `count` wide with its front `front` cells along the lane, placed there.
func _line(sim: FormationSimulation, count: int, front: float) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, "player", true)
	squad.order = SkirmishUnit.Order.HOLD
	squad.state = SkirmishSquad.State.HOLDING
	squad.front_distance = front / 64.0
	squad.fought = true  # meeting again after a fight
	FormationMarch.sync_units([squad])
	return squad


func _lead(squad: SkirmishSquad) -> void:
	squad.units[0].leadership = 1


func _form_up(sim: FormationSimulation) -> Array:
	var events := []
	FormationGroups.step(sim.squads(), 1, 7, events, 100)
	return events


func test_units_fallen_far_behind_go_on_as_a_group_of_the_same_command() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 4, 40.0)
	var behind: Array = [squad.units[2], squad.units[3]]
	var back := Vector2(BattleTuning.current().walk_lost + 4.0, 0)
	for unit in behind:
		UnitPlaces.put(squad, unit, UnitPlaces.of(squad, unit) - back)

	var events := []
	var next := GroupSplit.step(sim.squads(), 1, 7, events, 100)

	assert_int(next).is_equal(101)
	assert_int(sim.squads().size()).is_equal(2)
	var group: SkirmishSquad = sim.squads()[1]
	assert_int(group.units.size()).is_equal(2)
	assert_object(group.command).is_same(squad.command)
	assert_int(squad.units.size()).is_equal(2)
	for unit in behind:
		assert_object(unit.command).is_same(squad.command)
		assert_int(unit.squad_id).is_equal(group.id)


func test_no_one_splits_off_if_no_man_is_left_behind() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3, 40.0)
	for unit in squad.units:
		unit.traits["no_man_left_behind"] = 1
	var back := Vector2(BattleTuning.current().walk_lost + 4.0, 0)
	UnitPlaces.put(squad, squad.units[2], UnitPlaces.of(squad, squad.units[2]) - back)

	GroupSplit.step(sim.squads(), 1, 7, [], 100)

	assert_int(sim.squads().size()).is_equal(1)


func test_one_commands_groups_meeting_are_one_group_again() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 4, 40.0)
	var part := _line(sim, 2, 38.5)
	part.command = squad.command  # a group of the same command, split off before
	for unit in part.units:
		unit.command = squad.command

	_form_up(sim)

	assert_int(sim.squads().size()).is_equal(1)
	assert_object(sim.squads()[0]).is_same(squad)
	assert_int(squad.living().size()).is_equal(6)


func test_a_leader_and_a_leaderless_group_keep_their_own_orders() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var led := _line(sim, 2, 40.0)
	_lead(led)
	_line(sim, 4, 38.5)

	_form_up(sim)

	assert_int(sim.squads().size()).is_equal(2)


func test_a_leader_that_gathers_takes_in_a_leaderless_group() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var led := _line(sim, 2, 40.0)
	_lead(led)
	led.units[0].traits["gathers"] = 1
	var larger := _line(sim, 4, 38.5)

	var events := _form_up(sim)

	assert_int(sim.squads().size()).is_equal(1)
	assert_object(sim.squads()[0]).is_same(led)
	assert_int(led.living().size()).is_equal(6)
	for unit in led.units:
		assert_object(unit.command).is_same(led.command)
	assert_bool(events.any(func(e): return e["type"] == "merged")).is_true()
	assert_bool(larger.units.is_empty()).is_true()


func test_a_leader_that_joins_brings_its_group_into_the_others_command() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var joiner := _line(sim, 3, 40.0)
	_lead(joiner)
	joiner.units[0].traits["joins"] = 1
	var host := _line(sim, 2, 38.5)

	_form_up(sim)

	assert_int(sim.squads().size()).is_equal(1)
	assert_object(sim.squads()[0]).is_same(host)
	assert_int(host.living().size()).is_equal(5)


func test_forces_posted_near_each_other_that_have_not_fought_stay_apart() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var led := _line(sim, 2, 40.0)
	_lead(led)
	var other := _line(sim, 3, 38.5)
	led.fought = false
	other.fought = false

	_form_up(sim)

	assert_int(sim.squads().size()).is_equal(2)


func test_two_leaderless_groups_stay_apart_unless_one_merges() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var small := _line(sim, 2, 40.0)
	var large := _line(sim, 3, 38.5)

	_form_up(sim)
	assert_int(sim.squads().size()).is_equal(2)

	small.merges = true
	_form_up(sim)
	assert_int(sim.squads().size()).is_equal(1)
	assert_object(sim.squads()[0]).is_same(large)
	assert_int(large.living().size()).is_equal(5)


func test_two_led_groups_keep_their_own_and_units_go_back_where_they_started() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var first := _line(sim, 3, 40.0)
	var second := _line(sim, 3, 38.5)
	_lead(first)
	_lead(second)
	var wanderer: SkirmishUnit = first.units[2]
	wanderer.origin = second.command  # it started under the second's command

	_form_up(sim)

	assert_int(sim.squads().size()).is_equal(2)
	assert_bool(second.units.has(wanderer)).is_true()
	assert_object(wanderer.command).is_same(second.command)
	assert_int(first.living().size()).is_equal(2)


func test_the_same_meetings_decide_the_same_whatever_the_list_order() -> void:
	var outcomes := []
	for reverse in [false, true]:
		var sim := FormationSimulation.new(4.0, 0.1)
		var a := _line(sim, 2, 40.0)
		var b := _line(sim, 2, 38.5)
		var c := _line(sim, 2, 37.0)
		for squad in [a, b, c]:
			squad.merges = true
		if reverse:
			sim.squads().reverse()
		_form_up(sim)
		var left: Array = sim.squads().map(func(s): return s.living().size())
		left.sort()
		var kept: Array = sim.squads().map(func(s): return [a, b, c].find(s))
		kept.sort()
		outcomes.append([left, kept])
	assert_array(outcomes[0]).is_equal(outcomes[1])
