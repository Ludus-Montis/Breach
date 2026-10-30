extends GdUnitTestSuite
## The record's commands given to the field in the crate (specs/39-simulation-in-rust.md step
## 8c-ix, native field_play/actions.rs and record.rs): `BodyField.field_replay` reads a field
## log as FormationRecord.parse does and plays it as the digest does, each tick's commands
## given as FormationFieldActions.apply gives them (the kingdom's pursuit, a wave sent, a
## route's squads retreating, a wave rerouted, its downed tended), for FIELD_EXTRA ticks past
## the last, the waves sent together (`send A+B`) timed by rehearsal as FormationField's
## send_together times them (step 8c-x). The crate's whole state is then the one GDScript's
## replay leaves.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const Digest = preload("res://tools/formation_digest.gd")

## The builders' and the waves' events (DomainProduction, FormationProduction).
const PRODUCED := ["from_reserve", "built", "to_reserve", "wave_full", "departed"]
## The digest's field logs, blows rolled and not; 0 and 3 send the waves together.
const RUNS := [[0, true], [1, true], [2, false], [3, false], [4, true]]


## GDScript's field replayed as the digest's `_field` plays it: the crate's state, the ticks
## and the builders' and waves' events.
func _by_gdscript(record: String, rolls: bool) -> Array:
	var read := FormationFieldActions.parse(record)
	var field := FormationFieldActions.field(read["seed"], read["captained"])
	field.sim.blow_rolls = rolls
	var commands: Array = read["commands"]
	var last: int = commands.back()["tick"] if not commands.is_empty() else 0
	var produced := 0
	for tick in range(last + Digest.FIELD_EXTRA):
		for queued in commands:
			if queued["tick"] == tick:
				FormationFieldActions.apply(field, queued)
		var events := field.step()
		produced += events.filter(func(e): return e["type"] in PRODUCED).size()
	for squad in field.sim.squads():
		squad.living()
	return [field.sim._field.snapshot(0, 0), [last + Digest.FIELD_EXTRA, produced]]


## The crate's field replayed: its state and what `field_replay` returns.
func _by_crate(record: String, rolls: bool) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var at := PackedInt64Array([int(rolls), int(NativeKernels.threaded), Digest.FIELD_EXTRA])
	var played: PackedInt64Array = sim._field.field_replay(CrateContent.store(), record, at)
	return [sim._field.snapshot(0, 0), Array(played)]


func test_the_crate_replays_the_field_logs_to_the_state_gdscript_does() -> void:
	for run in RUNS:
		var record: String = Digest.FIELD_LOGS[run[0]]
		var by_gdscript := _by_gdscript(record, run[1])
		var by_crate := _by_crate(record, run[1])
		var label := "field%d r%s" % run
		assert_array(by_crate[1]).override_failure_message(label).is_equal(by_gdscript[1])
		var differs := "the crate's field differs from GDScript's: %s" % label
		var same: bool = by_crate[0] == by_gdscript[0]
		assert_bool(same).override_failure_message(differs).is_true()


func test_the_comparison_tells_one_retreat_from_another() -> void:
	var record: String = Digest.FIELD_LOGS[2]
	var by_gdscript := _by_gdscript(record, false)
	var by_crate := _by_crate(record.replace("229 retreat A", "229 retreat B"), false)
	assert_bool(by_crate[0] == by_gdscript[0]).is_false()


func test_what_is_not_a_record_is_not_replayed() -> void:
	var at := PackedInt64Array([1, 0, 10])
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var played: PackedInt64Array = sim._field.field_replay(CrateContent.store(), "not a record", at)
	assert_array(Array(played)).is_equal([-1])
