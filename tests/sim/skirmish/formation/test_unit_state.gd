extends GdUnitTestSuite
## A unit's state through its one writer, UnitState (specs/35-unit-store.md step 7a,
## Decision 147): each squad keeps its living and its downed (DOWNED or CARRIED) in
## `squad.units` order; a write, a join or a leaving changes them by the next read; a walk
## over a list is not changed by a write made during it; and a list handed out can't be
## changed by its caller.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	return unit


func _squad(count: int) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for index in range(count):
		members.append(_unit(index + 1))
	return SkirmishSquad.new(1, "player", 1, 0.0, count, members)


func _ids(units: Array) -> Array:
	return units.map(func(unit): return unit.id)


func test_the_lists_keep_the_squads_order() -> void:
	var squad := _squad(6)
	var units := squad.units
	UnitState.put(squad, units[4], SkirmishUnit.State.DOWNED)
	UnitState.put(squad, units[1], SkirmishUnit.State.CARRIED)
	UnitState.put(squad, units[3], SkirmishUnit.State.DEAD)
	UnitState.put(squad, units[5], SkirmishUnit.State.TAKEN)

	assert_array(_ids(squad.living())).is_equal([1, 3])
	assert_array(_ids(UnitState.downed(squad))).is_equal([2, 5])
	assert_int(units[4].state).is_equal(SkirmishUnit.State.DOWNED)


func test_a_write_changes_the_lists_by_the_next_read() -> void:
	var squad := _squad(3)
	assert_array(_ids(squad.living())).is_equal([1, 2, 3])
	UnitState.put(squad, squad.units[1], SkirmishUnit.State.DOWNED)
	assert_array(_ids(squad.living())).is_equal([1, 3])
	assert_array(_ids(UnitState.downed(squad))).is_equal([2])
	UnitState.put(squad, squad.units[1], SkirmishUnit.State.MOVING)
	assert_array(_ids(squad.living())).is_equal([1, 2, 3])
	assert_array(UnitState.downed(squad)).is_empty()


func test_a_walk_is_not_changed_by_a_write_during_it() -> void:
	var squad := _squad(4)
	var walked := []
	for unit in squad.living():
		walked.append(unit.id)
		if unit.id == 1:
			UnitState.put(squad, squad.units[2], SkirmishUnit.State.DEAD)
			UnitState.put(squad, unit, SkirmishUnit.State.DOWNED)
	assert_array(walked).is_equal([1, 2, 3, 4])
	assert_array(_ids(squad.living())).is_equal([2, 4])


func test_a_join_and_a_leaving_change_the_lists() -> void:
	var squad := _squad(2)
	assert_array(_ids(squad.living())).is_equal([1, 2])
	FormationCommand.enlist(squad, _unit(9))
	assert_array(_ids(squad.living())).is_equal([1, 2, 9])
	squad.units.erase(squad.units[0])
	UnitState.regather(squad)
	assert_array(_ids(squad.living())).is_equal([2, 9])
	squad.units.reverse()
	UnitState.regather(squad)
	assert_array(_ids(squad.living())).is_equal([9, 2])


func test_a_list_handed_out_cant_be_changed() -> void:
	var squad := _squad(2)
	assert_bool(squad.living().is_read_only()).is_true()
	assert_bool(UnitState.downed(squad).is_read_only()).is_true()
	assert_bool(squad.living().duplicate().is_read_only()).is_false()
