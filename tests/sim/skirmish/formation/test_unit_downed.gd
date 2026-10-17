extends GdUnitTestSuite
## The downed's and recovery's fields in the unit store (specs/35-unit-store.md step 7b,
## Decision 147): the part of a point regeneration has regained, the seconds a blow stopped
## it for, the seconds a downed unit has yet to lie, the part of a point its condition has
## drained, and the wounds the last blow that struck it would add. The columns round-trip
## and start blank on both backends and the native store holds them on its field; the
## recovery pass reads and writes them through its view, alike on both backends; a blow
## that lands writes its wounds and its halt to the store, and bury reads them back; and
## SkirmishUnit no longer declares them. The native half is skipped where the Rust core
## isn't built (native/build.sh).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")

## The new columns, each with a value unlike its blank.
const SET := {
	UnitColumns.REGENERATION_CARRY: 0.25,
	UnitColumns.REGENERATION_HALT: 1.5,
	UnitColumns.WAKE_LEFT: 4.5,
	UnitColumns.DRAIN_CARRY: 0.75,
	UnitColumns.LAST_WOUNDING: 2,
}


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [[NativeKernels.GDSCRIPT, NativeKernels.unit_store(null)]]
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_unit_downed: the Rust core isn't built - its half skipped")
	else:
		out.append([NativeKernels.RUST, NativeKernels.unit_store(field)])
	return out


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	unit.attributes = {"constitution": 10}
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
			assert_float(field.column_reals(UnitColumns.REGENERATION_CARRY)[slot]).is_equal(0.25)
			assert_float(field.column_reals(UnitColumns.REGENERATION_HALT)[slot]).is_equal(1.5)
			assert_float(field.column_reals(UnitColumns.WAKE_LEFT)[slot]).is_equal(4.5)
			assert_float(field.column_reals(UnitColumns.DRAIN_CARRY)[slot]).is_equal(0.75)
			assert_int(field.column_ints(UnitColumns.LAST_WOUNDING)[slot]).is_equal(2)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		for name: StringName in SET:
			(
				assert_that(columns.value(slot, name))
				. override_failure_message("%s %s" % [said, name])
				. is_equal(UnitColumns.BLANKS[name])
			)


## A standing regenerator whose regeneration a blow stopped counts the halt down; a downed
## unit in poor condition drains and counts down its time downed. The pass leaves the same
## columns on both backends.
func test_the_recovery_pass_reads_and_writes_the_store_alike_on_both_backends() -> void:
	var said := []
	for backend in _backends():
		var store: UnitStore = backend[1]
		var troll := _unit(1)
		var downed := _unit(2)
		troll.regeneration = 2.0
		var squad := _squad(1, [troll, downed], store)
		UnitHealth.put_one(squad, troll, 4)
		UnitHealth.put_one(squad, downed, 5)
		UnitState.put(squad, downed, SkirmishUnit.State.DOWNED)
		downed.wounded = 3
		var columns := store.columns
		columns.put(troll.slot, UnitColumns.REGENERATION_LEFT, 100.0)
		columns.put(troll.slot, UnitColumns.REGENERATION_HALT, 5.0)
		columns.put(downed.slot, UnitColumns.WAKE_LEFT, 999.0)
		for tick in 3:
			FormationRecovery.step([squad], 0.5, tick, 3, [])
		var row := []
		for unit in [troll, downed]:
			for name: StringName in SET:
				row.append(columns.value(unit.slot, name))
			row.append(UnitHealth.of_one(squad, unit))
		said.append(row)
		assert_float(columns.value(troll.slot, UnitColumns.REGENERATION_HALT)).is_equal(3.5)
		assert_int(UnitHealth.of_one(squad, troll)).is_equal(4)  # stopped: no gain
		assert_float(columns.value(downed.slot, UnitColumns.WAKE_LEFT)).is_equal(997.5)
	for row in said:
		assert_array(row).is_equal(said[0])


## A wounding blow that lands (rolls off: a hit at its full damage) writes the wounds it
## would add, and a blow of a type its target fears stops its regeneration, both in the
## store; bury then reads the wounds from it as the target is downed.
func test_a_landed_blow_writes_its_wounds_and_halt_to_the_store_for_bury() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var said: String = backend[0]
		var striker := _unit(1)
		striker.wounding = 2
		striker.melee_parts = [[3.0, "fire", true]]
		var target := _unit(2)
		target.regeneration_stops = ["fire"]
		target.faction_id = "the_kingdom"
		var ours := _squad(1, [striker], store)
		var theirs := _squad(2, [target], store)
		UnitHealth.put_one(ours, striker, 10)
		UnitHealth.put_one(theirs, target, 1)
		var blows := [[striker, target, 3, false]]
		BlowLanding.land(blows, [], [ours, theirs], null, [])
		var columns := store.columns
		(
			assert_int(columns.value(target.slot, UnitColumns.LAST_WOUNDING))
			. override_failure_message(said)
			. is_equal(2)
		)
		assert_float(columns.value(target.slot, UnitColumns.REGENERATION_HALT)).is_equal(
			BattleTuning.current().wounds_regeneration_halt
		)
		UnitHealth.put_one(theirs, target, 0)
		FormationDeaths.bury([ours, theirs], 1, [])
		assert_int(target.wounded).override_failure_message(said).is_equal(3)
		assert_float(columns.value(target.slot, UnitColumns.WAKE_LEFT)).is_equal(-1.0)


func test_the_downed_s_and_recovery_s_fields_are_the_store_s_alone() -> void:
	var unit := SkirmishUnit.new()
	for name in [
		"regeneration_carry", "regeneration_halt", "wake_left", "drain_carry", "last_wounding"
	]:
		assert_bool(name in unit).override_failure_message(name).is_false()  # removed
	assert_bool("wounded" in unit).is_true()  # event-only: its record's (Decision 147)
	assert_bool("wounding" in unit).is_true()
	assert_float(UnitColumns.BLANKS[UnitColumns.WAKE_LEFT]).is_equal(-1.0)  # not yet reckoned
