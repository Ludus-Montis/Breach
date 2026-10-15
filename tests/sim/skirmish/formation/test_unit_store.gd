extends GdUnitTestSuite
## The battle's unit store (specs/35-unit-store.md, Decision 142), step 0: slots. Both
## backends - UnitStoreGd, the reference, and UnitStoreNative on a BodyField - are held to
## one contract here, unmodified: dense slots, free slots reused lowest first, a slot freed
## only when its unit is in no list. A battle with deaths and a later wave gives every
## listed unit its own slot, the same on both engines, tick by tick, as units flee home and
## leave the lists; and slots given in
## another order change nothing. The native half is skipped where the Rust core isn't
## built (native/build.sh).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const TICKS := 450  # past the first routers fleeing home (from tick 409)
const WAVE_AT := 120  # a later wave, once the first blows have fallen


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [[NativeKernels.GDSCRIPT, NativeKernels.unit_store(null)]]
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_unit_store: the Rust core isn't built - its half skipped")
	else:
		out.append([NativeKernels.RUST, NativeKernels.unit_store(field)])
	return out


func test_slots_are_dense_and_free_slots_are_reused_lowest_first() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var given := [10, 11, 12, 13].map(func(unit_id): return store.enrol(unit_id))
		assert_array(given).override_failure_message(backend[0]).is_equal([0, 1, 2, 3])
		assert_int(store.enrol(12)).is_equal(2)  # a unit keeps its slot
		store.drop(13)
		store.drop(11)
		store.drop(99)  # not held: nothing
		assert_int(store.count()).is_equal(2)
		assert_int(store.slot_of(11)).is_equal(-1)
		assert_int(store.id_at(1)).is_equal(-1)
		assert_int(store.enrol(20)).is_equal(1)  # the lowest free slot
		assert_int(store.enrol(21)).is_equal(3)
		assert_int(store.enrol(22)).is_equal(4)
		assert_array(Array(store.ids())).is_equal([10, 20, 12, 21, 22])
		assert_int(store.id_at(store.slot_of(21))).is_equal(21)


func test_a_column_round_trips_and_a_reused_slot_starts_blank() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var slot := store.enrol(5)
		columns.put(slot, UnitColumns.POSITION, Vector2(3, 4))
		assert_vector(columns.value(slot, UnitColumns.POSITION)).is_equal(Vector2(3, 4))
		var other := store.enrol(6)
		var at: PackedVector2Array = columns.column(UnitColumns.POSITION)
		at[other] = Vector2(7, 8)
		columns.commit(UnitColumns.POSITION, at)
		assert_vector(columns.value(other, UnitColumns.POSITION)).is_equal(Vector2(7, 8))
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(backend[0]).is_equal(slot)
		assert_vector(columns.value(slot, UnitColumns.POSITION)).is_equal(Vector2.ZERO)


func test_where_a_unit_stands_is_the_store_s_alone() -> void:
	assert_bool("position" in SkirmishUnit.new()).is_false()  # removed, not forwarded
	assert_bool("slot" in SkirmishUnit.new()).is_true()


## Every listed unit's [slot, where it stands], by id; fails if two share a slot, the store
## holds an unlisted unit, or a unit's record names another slot.
func _slots(sim: FormationSimulation, label: String) -> Dictionary:
	var store := sim.store
	var at: PackedVector2Array = store.columns.column(UnitColumns.POSITION)
	var slots := {}
	var taken := {}
	for squad in sim.squads():
		assert_object(squad.store).override_failure_message(label).is_same(store)
		for unit in squad.units:
			var slot := store.slot_of(unit.id)
			assert_int(unit.slot).override_failure_message(label).is_equal(slot)
			assert_int(store.id_at(slot)).override_failure_message(label).is_equal(unit.id)
			assert_bool(taken.has(slot)).override_failure_message(label).is_false()
			taken[slot] = true
			slots[unit.id] = [slot, at[slot]]
	assert_int(store.count()).override_failure_message(label).is_equal(slots.size())
	return slots


## A 24-a-side clash on `engine`, a second grem wave sent at WAVE_AT; its slots each tick.
func _battle(engine: String) -> Array:
	NativeKernels.use(engine)
	var sim := FormationBench.clash(24, 8, 3)
	sim.blow_rolls = true
	NativeKernels.use(NativeKernels.GDSCRIPT)
	var grem: Resource = load("res://content/units/grem.tres")
	var wave := []
	for column in range(8):
		wave.append([grem, Vector2i(0, column)])
	var ticks := []
	var died := false
	var gone := false
	for tick in range(1, TICKS + 1):
		if tick == WAVE_AT:
			sim.spawn_squad(8, wave, "player", true)
		gone = gone or sim.step().any(func(e): return e["type"] == "fled_home")
		ticks.append(_slots(sim, "%s tick %d" % [engine, tick]))
		died = died or sim.squads().any(func(s): return s.units.any(func(u): return u.hp <= 0))
	assert_bool(died).override_failure_message("the battle needs deaths").is_true()
	assert_bool(gone).override_failure_message("the battle needs units leaving").is_true()
	return ticks


func test_a_battle_with_deaths_and_a_later_wave_gives_each_unit_its_own_slot() -> void:
	var engines := _backends().map(func(backend): return backend[0])
	var runs := engines.map(func(engine): return _battle(engine))
	for index in range(1, runs.size()):
		for tick in range(TICKS):
			for unit_id in runs[0][tick]:
				if runs[index][tick].get(unit_id) != runs[0][tick][unit_id]:
					var said := "%s first differs at tick %d, unit %d: [slot, position] %s, not %s"
					var theirs = runs[index][tick].get(unit_id)
					fail(said % [engines[index], tick + 1, unit_id, theirs, runs[0][tick][unit_id]])
					return


## The battle's units, by where they stand in their squads, as the reversed-lists tests
## read them.
func _state(sim: FormationSimulation) -> String:
	var rows := []
	var points: PackedVector2Array = sim.store.columns.column(UnitColumns.POSITION)
	for squad in sim.squads():
		for unit in squad.units:
			var at: Vector2 = points[unit.slot].snapped(Vector2(0.0001, 0.0001))
			rows.append([unit.id, unit.hp, unit.rank, unit.column, at, unit.bearing, squad.id])
	rows.sort()
	return str(rows)


func test_units_given_other_slots_play_the_same_battle() -> void:
	for engine in _backends().map(func(backend): return backend[0]):
		NativeKernels.use(engine)
		var usual := FormationBench.clash(16, 8, 2)
		var shuffled := FormationBench.clash(16, 8, 2)
		NativeKernels.use(NativeKernels.GDSCRIPT)
		var store := shuffled.store
		var stood := {}  # unit -> where it stands
		for squad in shuffled.squads():
			for unit in squad.units:
				stood[unit] = store.columns.value(unit.slot, UnitColumns.POSITION)
		var ids := Array(store.ids())
		for unit_id in ids:
			store.drop(unit_id)
		ids.reverse()
		for unit_id in ids:
			store.enrol(unit_id)  # every slot handed out the other way round
		for unit in stood:
			unit.slot = store.slot_of(unit.id)
			store.columns.put(unit.slot, UnitColumns.POSITION, stood[unit])
		assert_int(store.slot_of(ids[0])).is_equal(0)
		for tick in range(200):
			usual.step()
			shuffled.step()
			if _state(usual) != _state(shuffled):
				fail("%s: other slots first change the battle at tick %d" % [engine, tick + 1])
				return
