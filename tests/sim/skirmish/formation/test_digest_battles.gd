extends GdUnitTestSuite
## The digest's battles set up in the crate (specs/39-simulation-in-rust.md step 8c-vi, native
## battles.rs): after `BodyField.battle`, the crate's whole state is the state BattleTrials'
## mirrors, FormationBench.clash and FormationField (the field logs' field) leave it in - the
## squads, their units, routes and codes, the holds queued and the kingdom's fronts - and the
## simulation it names is the one they make.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")


## The mirror `scenario` as BattleTrials spawns it, its sides swapped if `swap`.
func _mirror(scenario: String, swap: bool) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var spawns := BattleTrials._mirror_spawns(scenario, sim)
	if swap:
		spawns.reverse()
	for spawn in spawns:
		spawn.call()
	return sim


## [battle name, its numbers, GDScript's simulation of it], for every battle the digest plays.
func _battles() -> Array:
	return [
		["mirror_headon", [0], _mirror("mirror_headon", false)],
		["mirror_flank", [0], _mirror("mirror_flank", false)],
		["mirror_flank", [1], _mirror("mirror_flank", true)],
		["clash", [40, 10], FormationBench.clash(40, 10, 1)],
		["clash", [96, 32], FormationBench.clash(96, 32, 1)],
		["clash", [24, 8, 8], FormationBench.clash(24, 8, 1, 8)],
		["field", [0], FormationFieldActions.field(1, false).sim],
		["field", [1], FormationFieldActions.field(1, true).sim],
	]


## The crate's state and the simulation named after it sets `name` up itself.
func _by_crate(name: String, numbers: Array) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var made: Dictionary = sim._field.battle(CrateContent.store(), name, PackedInt64Array(numbers))
	return [sim._field.snapshot(0, 0), made]


func test_the_crate_sets_up_each_digest_battle_as_gdscript_does() -> void:
	for battle in _battles():
		var sim: FormationSimulation = battle[2]
		var by_gdscript: PackedByteArray = sim._field.snapshot(0, 0)
		var by_crate := _by_crate(battle[0], battle[1])
		var label := "%s %s" % [battle[0], battle[1]]
		var differs := "the crate's %s differs from GDScript's" % label
		assert_bool(by_crate[0] == by_gdscript).override_failure_message(differs).is_true()
		var want := {
			"route_tiles": sim.route_length,
			"tick_seconds": sim.tick_seconds,
			"combat_width": sim.combat_width,
			"map": "field" if sim.terrain != null else "",
		}
		assert_dict(by_crate[1]).override_failure_message(label).is_equal(want)


func test_the_comparison_tells_a_captained_line_from_one_without() -> void:
	var captained: FormationSimulation = FormationFieldActions.field(1, true).sim
	var by_gdscript: PackedByteArray = captained._field.snapshot(0, 0)
	assert_bool(_by_crate("field", [0])[0] == by_gdscript).is_false()


func test_nothing_is_set_up_for_a_battle_that_isnt_one() -> void:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var made: Dictionary = sim._field.battle(CrateContent.store(), "lanes", PackedInt64Array())
	assert_dict(made).is_empty()
	assert_int(sim._field.squad_count()).is_equal(0)
