extends GdUnitTestSuite
## The battle's unit store (specs/35-unit-store.md, Decision 142), step 0: slots. Both
## backends - UnitStoreGd, the reference, and UnitStoreNative on a BodyField - are held to
## one contract here, unmodified: dense slots, free slots reused lowest first, a slot freed
## only when its unit is in no list. A battle with deaths and a later wave gives every
## listed unit its own slot, the same on both engines, tick by tick, as units flee home and
## leave the lists; and slots given in
## another order change nothing. Where each unit stands - its position, and its loose and
## fleeing places (`at`, `offset`) with a flag bit for each entry - is in the store's
## columns alone (steps 1a and 1b). The native half is skipped where the Rust core isn't
## built (native/build.sh).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

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


func test_the_loose_and_fleeing_places_round_trip_and_start_blank() -> void:
	var places := {
		UnitColumns.LOOSE_AT: Vector2(1, 2),
		UnitColumns.FLEE_OFFSET: Vector2(-3, 0.5),
		UnitColumns.FLAGS: UnitColumns.LOOSE | UnitColumns.FLEEING,
	}
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var slot := store.enrol(5)
		for name in places:
			assert_that(columns.value(slot, name)).override_failure_message(name).is_equal(
				UnitColumns.BLANKS[name]
			)
			columns.put(slot, name, places[name])
			assert_that(columns.value(slot, name)).override_failure_message(name).is_equal(
				places[name]
			)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(backend[0]).is_equal(slot)
		for name in places:
			assert_that(columns.value(slot, name)).is_equal(UnitColumns.BLANKS[name])


func test_the_native_store_s_columns_live_on_its_field() -> void:
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_unit_store: the Rust core isn't built - its half skipped")
		return
	var store := NativeKernels.unit_store(field)
	var slot := store.enrol(3)
	store.columns.put(slot, UnitColumns.LOOSE_AT, Vector2(4, 5))
	store.columns.put(slot, UnitColumns.FLAGS, UnitColumns.LOOSE)
	assert_vector(field.column_points(UnitColumns.LOOSE_AT)[slot]).is_equal(Vector2(4, 5))
	assert_int(field.column_flags()[slot]).is_equal(UnitColumns.LOOSE)
	assert_object(store.field()).is_same(field)


func test_a_sync_reads_where_each_unit_stands_from_the_store() -> void:
	for engine in _backends().map(func(backend): return backend[0]):
		if engine != NativeKernels.RUST:
			continue
		NativeKernels.use(engine)
		var sim := FormationBench.clash(8, 4, 3)
		NativeKernels.use(NativeKernels.GDSCRIPT)
		for _tick in range(30):
			sim.step()
		var field: Object = sim.store.field()
		var squads := sim.squads()
		BodyFieldSync.bodies(field, squads, sim.fight_seed)
		var flat := 0
		var points: PackedVector2Array = field.points()
		for squad in squads:
			for unit in squad.living():
				var at: Vector2 = UnitBodies.at(squad, unit)  # a router: its route point + offset
				assert_vector(points[flat]).override_failure_message(str(unit.id)).is_equal(at)
				flat += 1


func test_a_squad_keeps_each_unit_s_flag_bits_with_its_entries() -> void:
	var unit := SkirmishUnit.new()
	unit.id = 4
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, [unit])
	var columns := squad.store.columns
	SquadEntries.loosen(squad, unit, {"unit": unit, "goal": null}, Vector2(2, 3))
	assert_vector(columns.value(unit.slot, UnitColumns.LOOSE_AT)).is_equal(Vector2(2, 3))
	SquadEntries.flee(squad, unit, {"along": 1.0}, Vector2(0, 1))
	assert_vector(columns.value(unit.slot, UnitColumns.FLEE_OFFSET)).is_equal(Vector2(0, 1))
	var both := UnitColumns.LOOSE | UnitColumns.FLEEING
	assert_int(columns.value(unit.slot, UnitColumns.FLAGS)).is_equal(both)
	SquadEntries.unloose(squad, unit.id)
	assert_int(columns.value(unit.slot, UnitColumns.FLAGS)).is_equal(UnitColumns.FLEEING)
	SquadEntries.loosen(squad, unit, {"unit": unit, "goal": null}, Vector2(2, 3))
	SquadEntries.unloose_all(squad)
	SquadEntries.unflee_all(squad)
	assert_int(columns.value(unit.slot, UnitColumns.FLAGS)).is_equal(0)
	assert_bool(squad.loose.is_empty() and squad.fleeing.is_empty()).is_true()


func test_where_a_unit_stands_is_the_store_s_alone() -> void:
	assert_bool("position" in SkirmishUnit.new()).is_false()  # removed, not forwarded
	assert_bool("slot" in SkirmishUnit.new()).is_true()


## Every listed unit's [slot, where it stands, its flag bits, its loose and fleeing places]
## by id; fails if two share a slot, the store holds an unlisted unit, a unit's record
## names another slot, an entry still holds its place, or a flag bit disagrees with the
## entries.
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
			var said := "%s, unit %d" % [label, unit.id]
			slots[unit.id] = [slot, at[slot]] + _places(squad, unit, said)
	assert_int(store.count()).override_failure_message(label).is_equal(slots.size())
	return slots


## The unit's [flag bits, [its loose place, its fleeing place] (null where it has no
## entry)]; fails if an entry still holds its place or a bit disagrees with the entries.
func _places(squad: SkirmishSquad, unit: SkirmishUnit, said: String) -> Array:
	var columns := squad.store.columns
	var entries := [squad.loose.get(unit.id), squad.fleeing.get(unit.id)]
	var kept: bool = entries[0] != null and entries[0].has("at")
	kept = kept or (entries[1] != null and entries[1].has("offset"))
	assert_bool(kept).override_failure_message(said + ": an entry holds its place").is_false()
	var bits := (
		(UnitColumns.LOOSE if entries[0] != null else 0)
		| (UnitColumns.FLEEING if entries[1] != null else 0)
	)
	var flags: int = columns.value(unit.slot, UnitColumns.FLAGS)
	assert_int(flags).override_failure_message(said).is_equal(bits)
	var places := [
		columns.value(unit.slot, UnitColumns.LOOSE_AT) if entries[0] != null else null,
		columns.value(unit.slot, UnitColumns.FLEE_OFFSET) if entries[1] != null else null,
	]
	return [flags, places]


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
					var said := "%s first differs at tick %d, unit %d: [slot, position, flags, places] %s, not %s"
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
		var stood := {}  # unit -> each column's value for it
		for squad in shuffled.squads():
			for unit in squad.units:
				stood[unit] = UnitColumns.BLANKS.keys().map(
					func(name): return store.columns.value(unit.slot, name)
				)
		var ids := Array(store.ids())
		for unit_id in ids:
			store.drop(unit_id)
		ids.reverse()
		for unit_id in ids:
			store.enrol(unit_id)  # every slot handed out the other way round
		for unit in stood:
			unit.slot = store.slot_of(unit.id)
			for index in UnitColumns.BLANKS.size():
				store.columns.put(unit.slot, UnitColumns.BLANKS.keys()[index], stood[unit][index])
		assert_int(store.slot_of(ids[0])).is_equal(0)
		for tick in range(200):
			usual.step()
			shuffled.step()
			if _state(usual) != _state(shuffled):
				fail("%s: other slots first change the battle at tick %d" % [engine, tick + 1])
				return
