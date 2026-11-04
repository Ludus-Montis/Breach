extends GdUnitTestSuite
## UnitView (spec 35 step 6a, Decision 146): one typed view onto the unit store, a typed
## property for every column; a pass holds the columns it names and writes them in place,
## or reads them with no hold.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const ColumnsHeld = preload("res://sim/skirmish/formation/columns_held.gd")
const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


class _Squad:
	var store: UnitStore


func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	for backend in out:
		for unit_id in [1, 2, 3]:
			backend[1].enrol(unit_id)
	return out


func _squad(store: UnitStore) -> _Squad:
	var squad := _Squad.new()
	squad.store = store
	return squad


func test_every_column_has_a_typed_property_of_its_own_type() -> void:
	var properties := {}
	for property in UnitView.new().get_property_list():
		properties[property["name"]] = property["type"]
	var columns := UnitColumns.new()
	for name: StringName in UnitColumns.BLANKS:
		var said := "UnitView has no typed property for the column %s" % name
		assert_bool(properties.has(String(name))).override_failure_message(said).is_true()
		var own: int = typeof(columns.column(name))
		assert_int(properties.get(String(name), TYPE_NIL)).override_failure_message(said).is_equal(
			own
		)


func test_a_held_view_writes_in_place_and_release_commits() -> void:
	for backend in _backends():
		var squad := _squad(backend[1])
		var view := UnitView.hold([squad], [UnitColumns.ALONG, UnitColumns.HP])
		assert_object(UnitView.of(squad)).is_same(view)
		var along: PackedFloat64Array = view.along
		along[1] = 3.0
		assert_float(squad.store.columns.value(1, UnitColumns.ALONG)).is_equal(3.0)
		squad.store.columns.put(2, UnitColumns.ALONG, 6.0)
		assert_float(along[2]).is_equal(6.0)
		UnitView.release(view)
		assert_bool(squad.store.columns is ColumnsHeld).is_false()
		(
			assert_float(squad.store.columns.value(1, UnitColumns.ALONG))
			. override_failure_message(backend[0])
			. is_equal(3.0)
		)


func test_a_view_spans_one_store_and_each_store_held_gets_its_own() -> void:
	var first := _squad(NativeKernels.unit_store(null))
	var second := _squad(NativeKernels.unit_store(null))
	for squad in [first, second]:
		squad.store.enrol(1)
	var view := UnitView.hold([first, second], [UnitColumns.RANK])
	assert_object(view.store).is_same(first.store)
	assert_object(UnitView.of(second).store).is_same(second.store)
	var ranks: PackedInt32Array = UnitView.of(second).rank
	ranks[0] = 4
	UnitView.release(view)  # every store held with it
	assert_bool(second.store.columns is ColumnsHeld).is_false()
	assert_int(second.store.columns.value(0, UnitColumns.RANK)).is_equal(4)


func test_a_read_view_holds_nothing_and_shares_a_held_copy() -> void:
	for backend in _backends():
		var squad := _squad(backend[1])
		squad.store.columns.put(1, UnitColumns.RANK, 2)
		var read := UnitView.read(squad, [UnitColumns.RANK, UnitColumns.COLUMN])
		assert_bool(squad.store.columns is ColumnsHeld).is_false()
		assert_int(read.rank[1]).override_failure_message(backend[0]).is_equal(2)
		var held := UnitView.hold([squad], [UnitColumns.RANK])
		var ranks: PackedInt32Array = held.rank
		ranks[2] = 5
		assert_int(UnitView.read(squad, [UnitColumns.RANK]).rank[2]).is_equal(5)
		UnitView.release(held)


func test_keep_on_a_view_keeps_as_entry_keys_keeps() -> void:
	for backend in _backends():
		var squad := _squad(backend[1])
		var names := [UnitColumns.CAUGHT, UnitColumns.HELD_BY, UnitColumns.FANNED, UnitColumns.KEYS]
		var view := UnitView.hold([squad], names)
		view.keep(0, UnitColumns.CAUGHT, 2)
		view.keep(0, UnitColumns.HELD_BY, 7)
		view.keep(0, UnitColumns.FANNED, 1.5)
		view.keep(0, UnitColumns.HELD_BY, null)
		var columns := squad.store.columns
		EntryKeys.keep(columns, 1, UnitColumns.CAUGHT, 2)
		EntryKeys.keep(columns, 1, UnitColumns.HELD_BY, 7)
		EntryKeys.keep(columns, 1, UnitColumns.FANNED, 1.5)
		EntryKeys.keep(columns, 1, UnitColumns.HELD_BY, null)
		UnitView.release(view)
		for name in names:
			(
				assert_that(squad.store.columns.value(0, name))
				. override_failure_message("%s %s" % [backend[0], name])
				. is_equal(squad.store.columns.value(1, name))
			)


func test_reading_a_column_the_view_doesnt_hold_asserts() -> void:
	var squad := _squad(NativeKernels.unit_store(null))
	var view := UnitView.read(squad, [UnitColumns.RANK])
	await (
		assert_error(func(): return view.hp)
		. is_runtime_error("Assertion failed: " + UnitView.NOT_NAMED % UnitColumns.HP)
	)


func test_a_native_entry_asserts_no_hold_spans_it() -> void:
	var squad := _squad(NativeKernels.unit_store(null))
	var view := UnitView.hold([squad], [UnitColumns.HP])
	await (
		assert_error(func(): BodyFieldSync.scrum(null, [squad], 0))
		. is_runtime_error("Assertion failed: " + StoreHolds.SPANS)
	)
	UnitView.release(view)


func test_read_once_reads_a_store_once_into_its_views() -> void:
	var squad := _squad(NativeKernels.unit_store(null))
	var other := _squad(squad.store)
	var views := {}
	var view := UnitView.read_once(views, squad, ScrumReach.STANDING)
	assert_object(UnitView.read_once(views, other, ScrumReach.STANDING)).is_same(view)
	assert_int(views.size()).is_equal(1)


## Spec 35 step 6b: where a unit stands, and a router's point, read from a view as the
## scattered forms read them from the store.
func test_the_view_forms_of_where_a_unit_stands_match_the_scattered_ones() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var squad := SkirmishSquad.new(
			1, "player", 1, 0.0, 1, [] as Array[SkirmishUnit], null, store
		)
		var units := []
		for unit_id in [11, 12, 13]:
			var unit := SkirmishUnit.new()
			unit.id = unit_id
			unit.slot = store.admit(unit)
			squad.units.append(unit)
			UnitState.regather(squad)
			store.columns.put(unit.slot, UnitColumns.POSITION, Vector2(unit_id, 2))
			units.append(unit)
		SquadEntries.loosen(squad, units[1], Vector2(5, 6))
		SquadEntries.flee(squad, units[2], 7.5, Vector2(0, 1))
		var standing := UnitView.read(squad, ScrumReach.STANDING)
		for unit in units.slice(0, 2):
			(
				assert_vector(ScrumReach.at_in(standing, squad, unit))
				. override_failure_message(backend[0])
				. is_equal(ScrumReach.at(squad, unit))
			)
		var route := UnitView.read(squad, FormationRout.ROUTE)
		var router: SkirmishUnit = units[2]
		assert_vector(FormationRout.where_in(route, squad, router)).is_equal(
			FormationRout.where(squad, router.id)
		)
		assert_vector(FormationRout.route_point_in(route, squad, router)).is_equal(
			Vector2(7.5, 0.0)
		)
