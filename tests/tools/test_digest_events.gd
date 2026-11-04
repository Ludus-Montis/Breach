extends GdUnitTestSuite
## The digest's events in the crate (specs/39-simulation-in-rust.md step 8c-xii, native
## digest/events.rs): `BodyField.digest_battle` and `BodyField.digest_field` play a digest run
## in the crate and give each tick's record as tools/formation_digest.gd's `_record` hashes
## it, `var_to_bytes([events, state])`: the crate's log as CrateEvents decodes it, on the field
## after the builders' and waves' events, then the state (step 8c-xi), so a digest with nothing
## from Godot hashes what the Godot digest hashes.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const DigestState = preload("res://tools/digest_state.gd")
const Digest = preload("res://tools/formation_digest.gd")

## Kinds the runs must tell of, between them each field decoded its own way: an order, a
## band, a heading, a unit-led blow with its flank, a definition, a "joined" left out or
## not, and the builders' and the waves' events.
const TOLD := [
	"order_applied",
	"morale_band",
	"turning",
	"hit",
	"fled_home",
	"came_to",
	"built",
	"to_reserve",
	"from_reserve",
	"wave_full",
]

var _told := {}


## [battle name, its numbers, GDScript's simulation of it, seed, rolls, the after]: the
## digest's `_mirror` and `_clash` runs.
func _runs() -> Array:
	return [
		["mirror_headon", [0], _mirror("mirror_headon"), 1, true, -1],
		["mirror_flank", [0], _mirror("mirror_flank"), 2, false, -1],
		["clash", [40, 10], FormationBench.clash(40, 10, 1), 1, true, Digest.LIMIT],
	]


func _mirror(scenario: String) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	for spawn in BattleTrials._mirror_spawns(scenario, sim):
		spawn.call()
	return sim


## The digest's `_record` bytes of a tick, and the kinds of its events counted.
func _record(events: Array, sim: FormationSimulation) -> PackedByteArray:
	for event in events:
		_told[event["type"]] = true
	var state := DigestState.state(sim.squads(), sim.store)
	return var_to_bytes([DigestState.plain(events), state])


## formation_digest.gd's `_run` over GDScript's `sim`: each tick's record.
func _run_by_gdscript(sim: FormationSimulation, after: int) -> Array:
	var records := []
	var contact := -1
	var broken := -1
	for tick in range(Digest.LIMIT):
		var events := sim.step()
		records.append(_record(events, sim))
		var ticks := tick + 1
		if contact < 0 and events.any(func(e): return e["type"] == "engaged"):
			contact = ticks
		if after >= 0 and contact >= 0 and ticks - contact >= after:
			break
		if broken < 0 and not _both_stand(sim.squads()):
			broken = ticks
		if broken >= 0 and ticks - broken >= Digest.TAIL:
			break
	return records


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


## formation_digest.gd's `_field` over field log `index`: each tick's record.
func _field_by_gdscript(index: int, rolls: bool) -> Array:
	var read := FormationFieldActions.parse(Digest.FIELD_LOGS[index])
	var field := FormationFieldActions.field(read["seed"], read["captained"])
	field.sim.blow_rolls = rolls
	var commands: Array = read["commands"]
	var last: int = commands.back()["tick"] if not commands.is_empty() else 0
	var records := []
	for tick in range(last + Digest.FIELD_EXTRA):
		for queued in commands:
			if queued["tick"] == tick:
				FormationFieldActions.apply(field, queued)
		records.append(_record(field.step(), field.sim))
	return records


## Where two runs' records first part: "" if nowhere.
func _parted(label: String, by_gdscript: Array, by_crate: Array) -> String:
	for tick in range(mini(by_gdscript.size(), by_crate.size())):
		if by_gdscript[tick] != by_crate[tick]:
			return "%s: tick %d's records differ" % [label, tick + 1]
	if by_gdscript.size() != by_crate.size():
		return "%s: %d ticks, not %d" % [label, by_crate.size(), by_gdscript.size()]
	return ""


func _battle_by_crate(name: String, numbers: Array, at: Array) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var content := CrateContent.store()
	return sim._field.digest_battle(content, name, PackedInt64Array(numbers), PackedInt64Array(at))


func _field_by_crate(index: int, rolls: bool) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var at := PackedInt64Array([int(rolls), int(NativeKernels.threaded), Digest.FIELD_EXTRA])
	return sim._field.digest_field(CrateContent.store(), Digest.FIELD_LOGS[index], at)


func test_the_crate_records_each_tick_as_the_digest_does() -> void:
	_told = {}
	var parted := []
	for run in _runs():
		var sim: FormationSimulation = run[2]
		sim.fight_seed = run[3]
		sim.blow_rolls = run[4]
		var by_gdscript := _run_by_gdscript(sim, run[5])
		var at := [run[3], int(run[4]), int(NativeKernels.threaded), Digest.LIMIT, run[5]]
		var label := "%s %s s%d" % [run[0], run[1], run[3]]
		parted.append(_parted(label, by_gdscript, _battle_by_crate(run[0], run[1], at)))
	for index in range(Digest.FIELD_LOGS.size()):
		var rolls := index % 2 == 0
		var label := "field%d r%s" % [index, rolls]
		var by_gdscript := _field_by_gdscript(index, rolls)
		parted.append(_parted(label, by_gdscript, _field_by_crate(index, rolls)))
	assert_array(parted.filter(func(p): return p != "")).is_empty()
	var untold := TOLD.filter(func(kind): return not _told.has(kind))
	assert_array(untold).override_failure_message("no run told of %s" % [untold]).is_empty()


func test_the_comparison_tells_one_seed_from_another() -> void:
	var sim := _mirror("mirror_headon")
	sim.fight_seed = 1
	sim.blow_rolls = true
	var by_gdscript := _run_by_gdscript(sim, -1)
	var at := [2, 1, int(NativeKernels.threaded), Digest.LIMIT, -1]
	var by_crate := _battle_by_crate("mirror_headon", [0], at)
	assert_str(_parted("seeds", by_gdscript, by_crate)).is_not_empty()


func test_an_unknown_battle_or_a_non_record_gives_no_records() -> void:
	var at := [1, 1, 0, 10, -1]
	assert_array(_battle_by_crate("no_such_battle", [], at)).is_empty()
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var field_at := PackedInt64Array([1, 0, 10])
	assert_array(sim._field.digest_field(CrateContent.store(), "not a record", field_at)).is_empty()
