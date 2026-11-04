extends GdUnitTestSuite
## The digest's state read in the crate (specs/39-simulation-in-rust.md step 8c-xi, native
## digest/state.rs and digest/bytes.rs): `BodyField.digest_state(slots)` gives the bytes
## Godot's `var_to_bytes` gives DigestState.state - every listed squad's and unit's state as
## plain values, its records and its units' loose, fleeing and chasing entries among them -
## so a digest with nothing from Godot hashes what tools/formation_digest.gd hashes.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const DigestState = preload("res://tools/digest_state.gd")
const Digest = preload("res://tools/formation_digest.gd")

## Ticks each run plays: long enough for the routs, the pursuits and the downed taken.
const TICKS := 700


## Each tick's state by GDScript and by the crate, [label, tick, GDScript's, the crate's
## length], at the first tick they differ ([] if none); and how many ticks held each kind of
## record.
func _compare(label: String, sim: FormationSimulation, step: Callable, seen: Dictionary) -> Array:
	for tick in range(1, TICKS + 1):
		step.call()
		var squads: Array = sim.squads()
		var state := DigestState.state(squads, sim.store)
		_count(state, seen)
		var slots := PackedInt64Array(squads.map(func(s): return s.slot))
		var by_gdscript := var_to_bytes(state)
		var by_crate: PackedByteArray = sim._field.digest_state(slots)
		if by_crate != by_gdscript:
			return [label, tick, var_to_str(state).left(2000), by_crate.size()]
	return []


## Counts the ticks whose state holds each record: flank contacts, a stance, a pursuit, a
## withdrawal, and a unit loose, fleeing or chasing. No digest run holds a stance or a chaser
## (test_a_stance_and_chasers_set_by_hand_are_read_as_godot_encodes_them).
func _count(state: Array, seen: Dictionary) -> void:
	var held := {}
	for squad in state:
		var records: Array = squad[2][1]
		for index in range(4):
			if records[index][0] > 0:
				held[["flanks", "stance", "pursuit", "withdraw"][index]] = true
		for unit in squad[2][2]:
			for index in range(3):
				if unit[1][3][index] != null:
					held[["loose", "fleeing", "chaser"][index]] = true
	for kind in held:
		seen[kind] = seen.get(kind, 0) + 1


func test_the_crate_reads_the_digest_state_godot_encodes() -> void:
	var seen := {}
	var differs := []
	for scenario in ["mirror_headon", "mirror_flank"]:
		var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
		sim.fight_seed = 1
		sim.blow_rolls = true
		for spawn in BattleTrials._mirror_spawns(scenario, sim):
			spawn.call()
		if differs.is_empty():
			differs = _compare(scenario, sim, func(): sim.step(), seen)
	for index in [2, 3]:
		var read := FormationFieldActions.parse(Digest.FIELD_LOGS[index])
		var field := FormationFieldActions.field(read["seed"], read["captained"])
		var commands: Array = read["commands"]
		var at := [0]
		var step := func():
			for queued in commands:
				if queued["tick"] == at[0]:
					FormationFieldActions.apply(field, queued)
			field.step()
			at[0] += 1
		if differs.is_empty():
			differs = _compare("field%d" % index, field.sim, step, seen)
	var clash := FormationBench.clash(40, 10, 1)
	if differs.is_empty():
		differs = _compare("clash40", clash, func(): clash.step(), seen)
	assert_array(differs).is_empty()
	var kinds := ["flanks", "pursuit", "withdraw", "loose", "fleeing"]
	var unseen := kinds.filter(func(k): return not seen.has(k))
	assert_array(unseen).override_failure_message("no tick held %s" % [unseen]).is_empty()


func test_the_squads_are_read_in_the_order_listed() -> void:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	for spawn in BattleTrials._mirror_spawns("mirror_headon", sim):
		spawn.call()
	sim.step()
	var squads: Array = sim.squads()
	var slots := PackedInt64Array(squads.map(func(s): return s.slot))
	slots.reverse()
	var by_crate: PackedByteArray = sim._field.digest_state(slots)
	assert_bool(by_crate == var_to_bytes(DigestState.state(squads, sim.store))).is_false()


func test_a_stance_and_chasers_set_by_hand_are_read_as_godot_encodes_them() -> void:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	for spawn in BattleTrials._mirror_spawns("mirror_headon", sim):
		spawn.call()
	sim.step()
	var squads: Array = sim.squads()
	squads[0].stance = {"anchor": Vector2(3.5, 1.25), "heading": 0.1, "foe": squads[1].id}
	var units: Array = squads[1].units
	squads[1].chase(units[0], squads[0].id, Vector2(2.0, 0.3), 6.5)
	squads[1].chase(units[1], squads[0].id, Vector2(1.0, 1.0), 4.0)
	squads[1].chase_back(units[1].id)
	var slots := PackedInt64Array(squads.map(func(s): return s.slot))
	var by_crate: PackedByteArray = sim._field.digest_state(slots)
	assert_bool(by_crate == var_to_bytes(DigestState.state(squads, sim.store))).is_true()
