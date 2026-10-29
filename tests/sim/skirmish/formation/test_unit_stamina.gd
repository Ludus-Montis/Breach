extends GdUnitTestSuite
## Each unit's stamina, pace and exertion in the unit store (specs/35-unit-store.md step 6c,
## Decisions 142 and 146): its stamina, its speed this tick, whether it exerted itself, its
## breather, how far into a run it is, whether it ran last tick and its condition. The
## columns round-trip and start blank and the native store holds them on its field; a unit
## entering a squad starts at its full stamina and its fresh pace; the stamina pass reads
## and writes them through its view; and SkirmishUnit no longer declares them.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")

## The new columns, each with a value unlike its blank.
const SET := {
	UnitColumns.STAMINA: 42.5,
	UnitColumns.SPEED: 1.25,
	UnitColumns.EXERTED: 1,
	UnitColumns.BREATHER: 0.75,
	UnitColumns.RUN_BUILD: -0.5,
	UnitColumns.WAS_RUNNING: 1,
	UnitColumns.CONDITION: 0.5,
}


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	return out


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	unit.max_stamina = 100.0
	unit.fresh_speed = 1.0
	return unit


func _squad(squad_id: int, members: Array, store: UnitStore) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(squad_id, "player", 1, 0.0, members.size(), typed, null, store)


func test_the_columns_round_trip_and_a_new_slot_starts_blank() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var said: String = backend[0]
		var slot := store.enrol(5)
		for name: StringName in SET:
			(
				assert_that(columns.value(slot, name))
				. override_failure_message("%s %s" % [said, name])
				. is_equal(UnitColumns.BLANKS[name])
			)
			columns.put(slot, name, SET[name])
		for name: StringName in SET:
			assert_that(columns.value(slot, name)).override_failure_message(said).is_equal(
				SET[name]
			)
			assert_that(columns.column(name)[slot]).override_failure_message(said).is_equal(
				SET[name]
			)
		var field: Object = store.field()
		if field != null:  # the native store holds them on its field
			assert_float(field.column_reals(UnitColumns.STAMINA)[slot]).is_equal(42.5)
			assert_float(field.column_reals(UnitColumns.SPEED)[slot]).is_equal(1.25)
			assert_int(field.column_ints(UnitColumns.EXERTED)[slot]).is_equal(1)
			assert_float(field.column_reals(UnitColumns.CONDITION)[slot]).is_equal(0.5)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		for name: StringName in SET:
			(
				assert_that(columns.value(slot, name))
				. override_failure_message("%s %s" % [said, name])
				. is_equal(UnitColumns.BLANKS[name])
			)


func test_a_unit_entering_its_squad_starts_at_its_full_stamina_and_fresh_pace() -> void:
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.speed = 1.5
	unit_def.constitution = 8
	var unit := FormationUnits.make(unit_def, "player", 1, 1)
	var squad := _squad(1, [unit], NativeKernels.unit_store(null))
	FormationUnits.enter(squad, [unit], [[unit_def, Vector2i.ZERO]])
	var columns := squad.store.columns
	var most := 8 * BattleTuning.current().stamina_per_constitution
	assert_float(unit.max_stamina).is_equal(most)
	assert_float(columns.value(unit.slot, UnitColumns.STAMINA)).is_equal(most)
	assert_float(columns.value(unit.slot, UnitColumns.SPEED)).is_equal(unit.fresh_speed)
	assert_float(unit.fresh_speed).is_greater(0.0)
	assert_float(columns.value(unit.slot, UnitColumns.CONDITION)).is_equal(1.0)


## A routing squad's units run: they build up their run, spend stamina and so don't recover
## this tick, in the store.
func test_the_stamina_pass_reads_and_writes_the_store() -> void:
	var said := []
	for backend in _backends():
		var store: UnitStore = backend[1]
		var runner := _unit(1)
		var resting := _unit(2)
		var running := _squad(1, [runner], store)
		running.state = SkirmishSquad.State.ROUTING
		var still := _squad(2, [resting], store)
		still.state = SkirmishSquad.State.FIGHTING
		UnitHealth.put_one(running, runner, 10)
		UnitHealth.put_one(still, resting, 10)
		var columns := store.columns
		columns.put(runner.slot, UnitColumns.STAMINA, 100.0)
		columns.put(resting.slot, UnitColumns.STAMINA, 50.0)
		for tick in 3:
			FormationStamina.step([running, still], 0.5)
		var row := []
		for unit in [runner, resting]:
			for name: StringName in SET:
				row.append(columns.value(unit.slot, name))
		said.append(row)
		var tuning := BattleTuning.current()
		assert_float(columns.value(runner.slot, UnitColumns.STAMINA)).is_less(100.0)
		assert_int(columns.value(runner.slot, UnitColumns.WAS_RUNNING)).is_equal(1)
		assert_float(columns.value(runner.slot, UnitColumns.RUN_BUILD)).is_equal(1.5)
		assert_float(columns.value(runner.slot, UnitColumns.SPEED)).is_greater(1.0)
		assert_int(columns.value(runner.slot, UnitColumns.EXERTED)).is_equal(0)  # reset after
		assert_float(columns.value(resting.slot, UnitColumns.BREATHER)).is_equal(1.5)
		var stage := FormationStamina.stage_at(
			columns.value(resting.slot, UnitColumns.STAMINA), 100.0
		)
		assert_float(columns.value(resting.slot, UnitColumns.SPEED)).is_equal_approx(
			tuning.stamina_pace[stage], 0.0001
		)
	for row in said:
		assert_array(row).is_equal(said[0])


func test_stamina_pace_and_exertion_are_the_store_s_alone() -> void:
	var unit := SkirmishUnit.new()
	for name in ["stamina", "speed", "exerted", "breather", "run_build", "was_running"]:
		assert_bool(name in unit).override_failure_message(name).is_false()  # removed
	assert_bool("condition" in unit).is_false()
	assert_bool("fresh_speed" in unit).is_true()  # its pace fresh: its record's
	assert_bool("max_stamina" in unit).is_true()


## The pass is the crate's (spec 39 step 8b5-xxxi): a squad its command hurries runs, read
## there, and a unit made by hand, with no fresh speed, takes its speed as fresh, in its own
## copy as well as the crate's.
func test_the_crate_pass_reads_the_hurry_and_gives_a_unit_made_by_hand_its_speed() -> void:
	var store := NativeKernels.unit_store(null)
	var runner := _unit(1)
	var made := _unit(2)
	var squad := _squad(1, [runner, made], store)
	squad.state = SkirmishSquad.State.MOVING
	squad.hurry = true
	made.fresh_speed = 0.0
	var columns := store.columns
	for unit in [runner, made]:
		columns.put(unit.slot, UnitColumns.STAMINA, 100.0)
	columns.put(made.slot, UnitColumns.SPEED, 2.0)
	FormationStamina.step([squad], 0.5)
	var said := [columns.value(runner.slot, UnitColumns.STAMINA) < 100.0, made.fresh_speed]
	assert_array(said).is_equal([true, 2.0])
