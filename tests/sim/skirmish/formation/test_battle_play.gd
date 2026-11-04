extends GdUnitTestSuite
## A digest battle played end to end in the crate (specs/39-simulation-in-rust.md step
## 8c-vii, native play.rs): `BodyField.battle_play` sets a battle up, plays it tick by tick
## as FormationSimulation.step does with nothing from GDScript, and stops it by the digest's
## rules (formation_digest.gd's `_run`). It stops on the same tick as GDScript's run, and the
## crate's whole state is then the one GDScript's simulation leaves.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

## formation_digest.gd's TAIL.
const TAIL := 150


## The mirror `scenario` as BattleTrials spawns it, its sides swapped if `swap`.
func _mirror(scenario: String, swap: bool) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var spawns := BattleTrials._mirror_spawns(scenario, sim)
	if swap:
		spawns.reverse()
	for spawn in spawns:
		spawn.call()
	return sim


## [battle name, its numbers, GDScript's simulation of it, seed, rolls, limit, after]: a
## side broken, then the tail; ticks past contact; the limit, on the field's ground.
func _runs() -> Array:
	return [
		["mirror_headon", [0], _mirror("mirror_headon", false), 1, true, 1500, -1],
		["mirror_flank", [1], _mirror("mirror_flank", true), 2, false, 1500, -1],
		["clash", [40, 10], FormationBench.clash(40, 10, 1), 1, true, 1500, 30],
		["field", [1], FormationFieldActions.field(1, true).sim, 1, true, 60, -1],
	]


## formation_digest.gd's `_run` over GDScript's `sim`: the ticks it plays.
func _by_gdscript(sim: FormationSimulation, limit: int, after: int) -> int:
	var contact := -1
	var broken := -1
	var ticks := 0
	for tick in range(limit):
		var events := sim.step()
		ticks = tick + 1
		if contact < 0 and events.any(func(e): return e["type"] == "engaged"):
			contact = ticks
		if after >= 0 and contact >= 0 and ticks - contact >= after:
			break
		if broken < 0 and not _both_stand(sim.squads()):
			broken = ticks
		if broken >= 0 and ticks - broken >= TAIL:
			break
	return ticks


func _both_stand(squads: Array) -> bool:
	for faction in BattleTrials.FACTIONS:
		var stands := func(s):
			return (
				s.faction_id == faction
				and not s.is_destroyed()
				and s.state != SkirmishSquad.State.ROUTING
			)
		if not squads.any(stands):
			return false
	return true


## The crate's state and the ticks played after it plays `name` itself.
func _by_crate(name: String, numbers: Array, at: Array) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var played: int = sim._field.battle_play(
		CrateContent.store(), name, PackedInt64Array(numbers), PackedInt64Array(at)
	)
	return [sim._field.snapshot(0, 0), played]


func test_the_crate_plays_each_battle_to_the_tick_and_state_gdscript_does() -> void:
	for run in _runs():
		var sim: FormationSimulation = run[2]
		sim.fight_seed = run[3]
		sim.blow_rolls = run[4]
		var ticks := _by_gdscript(sim, run[5], run[6])
		var at := [run[3], int(run[4]), int(NativeKernels.threaded), run[5], run[6]]
		var by_crate := _by_crate(run[0], run[1], at)
		var label := "%s %s s%d" % [run[0], run[1], run[3]]
		assert_int(by_crate[1]).override_failure_message(label).is_equal(ticks)
		var differs := "the crate's %s differs from GDScript's after %d ticks" % [label, ticks]
		var same: bool = by_crate[0] == sim._field.snapshot(0, 0)
		assert_bool(same).override_failure_message(differs).is_true()


func test_the_comparison_tells_one_seed_from_another() -> void:
	var sim := _mirror("mirror_headon", false)
	sim.fight_seed = 1
	sim.blow_rolls = true
	var ticks := _by_gdscript(sim, 200, -1)
	var by_crate := _by_crate("mirror_headon", [0], [2, 1, int(NativeKernels.threaded), 200, -1])
	assert_int(by_crate[1]).is_equal(ticks)
	assert_bool(by_crate[0] == sim._field.snapshot(0, 0)).is_false()


func test_nothing_is_played_for_a_battle_that_isnt_one() -> void:
	var played := _by_crate("lanes", [], [1, 1, 0, 10, -1])
	assert_int(played[1]).is_equal(-1)
