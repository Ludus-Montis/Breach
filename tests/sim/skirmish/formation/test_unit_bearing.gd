extends GdUnitTestSuite
## The way each unit faces in the unit store (specs/35-unit-store.md step 3): the bearing
## column round-trips, a slot newly given faces east as a new unit did,
## the native store holds it on its field, and SkirmishUnit no longer declares it. The
## battle's per-tick comparison of both engines (test_unit_store.gd) compares it too.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EAST := 90.0


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	return out


func test_a_unit_s_bearing_round_trips_and_a_new_slot_faces_east() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var said: String = backend[0]
		var slot := store.enrol(5)
		var facing: float = columns.value(slot, UnitColumns.BEARING)
		assert_float(facing).override_failure_message(said).is_equal(EAST)
		columns.put(slot, UnitColumns.BEARING, 237.5)
		assert_float(columns.value(slot, UnitColumns.BEARING)).is_equal(237.5)
		var field: Object = store.field()
		if field != null:  # the native store's bearing lives on its field
			assert_float(field.column_reals(UnitColumns.BEARING)[slot]).is_equal(237.5)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		facing = columns.value(slot, UnitColumns.BEARING)
		assert_float(facing).override_failure_message(said).is_equal(EAST)


func test_the_bearing_is_the_store_s_alone() -> void:
	assert_bool("bearing" in SkirmishUnit.new()).is_false()  # removed, not forwarded
