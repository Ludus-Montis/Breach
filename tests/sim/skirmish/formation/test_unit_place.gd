extends GdUnitTestSuite
## Each unit's place in its squad's frame in the unit store (specs/35-unit-store.md step
## 4a): the rank and column columns round-trip on both backends, a slot newly given starts
## at the front left (0, 0), the native store holds them on its field, and SkirmishUnit no
## longer declares them. A battle's units start where its layout places them. The battle's
## per-tick comparison of both engines (test_unit_store.gd) compares them too. The native
## half is skipped where the Rust core isn't built (native/build.sh).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [[NativeKernels.GDSCRIPT, NativeKernels.unit_store(null)]]
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_unit_place: the Rust core isn't built - its half skipped")
	else:
		out.append([NativeKernels.RUST, NativeKernels.unit_store(field)])
	return out


func test_a_unit_s_place_round_trips_and_a_new_slot_starts_at_the_front_left() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var said: String = backend[0]
		var slot := store.enrol(5)
		assert_vector(Vector2(UnitPlace.at(columns, slot))).override_failure_message(said).is_equal(
			Vector2.ZERO
		)
		UnitPlace.set_at(columns, slot, Vector2i(3, -2))
		assert_int(columns.value(slot, UnitColumns.RANK)).is_equal(3)
		assert_int(columns.value(slot, UnitColumns.COLUMN)).is_equal(-2)
		columns.put(slot, UnitColumns.COLUMN, 7)
		assert_vector(Vector2(UnitPlace.at(columns, slot))).is_equal(Vector2(3, 7))
		var field: Object = store.field()
		if field != null:  # the native store's place lives on its field
			assert_int(field.column_ints(UnitColumns.RANK)[slot]).is_equal(3)
			assert_int(field.column_ints(UnitColumns.COLUMN)[slot]).is_equal(7)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		assert_vector(Vector2(UnitPlace.at(columns, slot))).override_failure_message(said).is_equal(
			Vector2.ZERO
		)


func test_a_spawned_unit_stands_where_its_layout_places_it() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var grem: Resource = CrateContent.unit("grem")
	var places := [Vector2i(0, 0), Vector2i(0, 1), Vector2i(1, 0), Vector2i(2, 1)]
	var squad = sim.spawn_squad(2, places.map(func(place): return [grem, place]), "player", true)
	for index in places.size():
		var unit: SkirmishUnit = squad.units[index]
		var place: Vector2i = UnitPlace.at(sim.store.columns, unit.slot)
		assert_vector(Vector2(place)).is_equal(Vector2(places[index]))


func test_the_place_is_the_store_s_alone() -> void:
	var unit := SkirmishUnit.new()
	assert_bool("rank" in unit).is_false()  # removed, not forwarded
	assert_bool("column" in unit).is_false()


## A pass over many units reads the places once (a view of UnitView.PLACES) and each unit's
## from it (UnitPlace.in_view), on both backends: the same places as reading the store.
func test_a_view_of_places_reads_each_unit_s_place_as_the_store_holds_it() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var squad := {"store": store}  # anything holding the store
		var units := []
		for unit_id in [3, 4, 5]:
			var unit := SkirmishUnit.new()
			unit.id = unit_id
			unit.slot = store.enrol(unit_id)
			UnitPlace.put(squad, unit, Vector2i(unit_id - 3, 7 - unit_id))
			units.append(unit)
		var view := UnitView.read(squad, UnitView.PLACES)
		for unit in units:
			var place := Vector2(UnitPlace.of(squad, unit))
			assert_vector(Vector2(UnitPlace.in_view(view, unit))).is_equal(place)
