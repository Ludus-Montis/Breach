extends GdUnitTestSuite
## ColumnsHeld and StoreHolds (fix/rout-store-reads): a pass holds the columns it reads unit
## by unit, so each scattered read and write is a local one, and every reader in the pass
## sees the same values; releasing commits them. The typed view a pass takes of a hold is
## UnitView's (test_unit_view.gd).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


func _backends() -> Array:
	var out := [[NativeKernels.RUST, NativeKernels.unit_store(null)]]
	for backend in out:
		for unit_id in [1, 2, 3]:
			backend[1].enrol(unit_id)
	return out


func test_a_held_write_is_read_back_in_the_pass_and_committed_on_release() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var inner := store.columns
		StoreHolds.hold_store(store, [UnitColumns.ALONG])
		store.columns.put(1, UnitColumns.ALONG, 4.5)
		assert_float(store.columns.value(1, UnitColumns.ALONG)).is_equal(4.5)
		StoreHolds.release([store])
		assert_object(store.columns).is_same(inner)
		(
			assert_float(inner.value(1, UnitColumns.ALONG))
			. override_failure_message(backend[0])
			. is_equal(4.5)
		)


func test_a_column_not_held_passes_straight_through() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var inner := store.columns
		StoreHolds.hold_store(store, [UnitColumns.ALONG])
		store.columns.put(2, UnitColumns.CAUGHT, 3)
		(
			assert_int(inner.value(2, UnitColumns.CAUGHT))
			. override_failure_message(backend[0])
			. is_equal(3)
		)
		StoreHolds.release([store])


func test_a_held_column_fetched_whole_carries_the_writes_and_a_commit_replaces_it() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		StoreHolds.hold_store(store, [UnitColumns.FLEE_OFFSET])
		store.columns.put(0, UnitColumns.FLEE_OFFSET, Vector2(1, 2))
		var offsets: PackedVector2Array = store.columns.column(UnitColumns.FLEE_OFFSET)
		assert_object(offsets[0]).is_equal(Vector2(1, 2))
		offsets[2] = Vector2(5, 6)
		store.columns.commit(UnitColumns.FLEE_OFFSET, offsets)
		assert_object(store.columns.value(2, UnitColumns.FLEE_OFFSET)).is_equal(Vector2(5, 6))
		StoreHolds.release([store])
		var after: PackedVector2Array = store.columns.column(UnitColumns.FLEE_OFFSET)
		assert_object(after[0]).override_failure_message(backend[0]).is_equal(Vector2(1, 2))
		assert_object(after[2]).override_failure_message(backend[0]).is_equal(Vector2(5, 6))


func test_a_slot_opened_in_the_pass_keeps_earlier_writes_and_starts_blank() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		StoreHolds.hold_store(store, [UnitColumns.ALONG, UnitColumns.KEYS])
		store.columns.put(1, UnitColumns.ALONG, 2.5)
		var slot := store.enrol(9)  # opens the new slot's columns
		assert_float(store.columns.value(slot, UnitColumns.ALONG)).is_equal(0.0)
		store.columns.put(slot, UnitColumns.KEYS, 64)
		StoreHolds.release([store])
		(
			assert_float(store.columns.value(1, UnitColumns.ALONG))
			. override_failure_message(backend[0])
			. is_equal(2.5)
		)
		(
			assert_int(store.columns.value(slot, UnitColumns.KEYS))
			. override_failure_message(backend[0])
			. is_equal(64)
		)


func test_releasing_a_store_not_held_changes_nothing() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var inner := store.columns
		StoreHolds.release([store])
		assert_object(store.columns).is_same(inner)
