extends GdUnitTestSuite
## FormationCommand (spec 30 round 3): a formation's orders live in its command; a unit
## carries the command it follows and the one it started under; a stray keeps its
## command's orders; a unit taken into another group follows that group's command.

const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const RECORD := (
	"record 1 seed 109563 captain on\n0 player auto A on\n0 player auto B on\n"
	+ "0 player tend A carry\n100 player send A+B"
)


func _line(sim: FormationSimulation, count: int, at_player_end: bool) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var faction := "player" if at_player_end else "the_kingdom"
	return sim.spawn_squad(count, placements, faction, at_player_end)


func test_a_spawned_squad_has_its_own_command_its_units_follow() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var first := _line(sim, 3, true)
	var second := _line(sim, 3, false)

	assert_bool(first.command != second.command).is_true()
	assert_object(first.command.route).is_same(first.route)
	for unit in first.units:
		assert_object(unit.command).is_same(first.command)
		assert_object(unit.origin).is_same(first.command)


func test_a_squads_standing_orders_are_its_commands() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 2, true)

	squad.hurry = true
	squad.tends = "carry"
	squad.merges = true
	squad.pursues = false

	assert_bool(squad.command.hurry).is_true()
	assert_str(squad.command.tends).is_equal("carry")
	assert_bool(squad.command.merges).is_true()
	assert_bool(squad.command.pursues).is_false()


func test_a_stray_keeps_its_command_and_shares_its_orders() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3, true)
	var unit: SkirmishUnit = squad.units[1]

	var squads := [squad]
	FormationStrays.adopt([[unit, squad]], squads, 9, 0)
	var stray: SkirmishSquad = squads[1]
	squad.hurry = true

	assert_object(stray.command).is_same(squad.command)
	assert_object(unit.command).is_same(squad.command)
	assert_bool(stray.hurry).is_true()


func test_a_unit_taken_into_another_group_follows_its_command_and_keeps_its_origin() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var own := _line(sim, 2, true)
	var other := _line(sim, 2, true)
	var unit: SkirmishUnit = own.units[0]

	own.units.erase(unit)
	UnitState.regather(own)
	FormationCommand.enlist(other, unit)

	assert_object(unit.command).is_same(other.command)
	assert_object(unit.origin).is_same(own.command)
	assert_int(unit.squad_id).is_equal(other.id)


func test_in_a_whole_battle_every_unit_follows_its_squads_command() -> void:
	var field = FormationFieldActions.replay(RECORD, 0)
	for _tick in 600:
		field.sim.step()
		for squad in field.sim.squads():
			for unit in squad.units:
				assert_object(unit.command).is_same(squad.command)
				assert_object(unit.origin).is_not_null()


func test_the_crate_holds_a_command_s_orders() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _line(sim, 3, true)
	var command := squad.command
	var field: Object = command.field
	command.hurry = true
	command.pursues = false
	command.tends = "carry"
	command.home_distance = 6.5
	assert_bool(field.command_gives(command.slot, FormationCommand.HURRY)).is_true()
	assert_bool(field.command_gives(command.slot, FormationCommand.PURSUES)).is_false()
	assert_bool(field.command_gives(command.slot, FormationCommand.TAKES)).is_true()
	assert_str(field.command_tends(command.slot)).is_equal("carry")
	assert_float(field.command_home(command.slot)).is_equal(6.5)
	assert_bool(squad.pursues).is_false()  # its groups read it there
