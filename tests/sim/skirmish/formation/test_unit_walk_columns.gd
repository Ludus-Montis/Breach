extends GdUnitTestSuite
## The rest of a unit's per-tick fields in the unit store (specs/35-unit-store.md step 7c,
## Decision 147): the unit it fights, how far along its route it stands, the last place it
## could stand, whether it is on a climb face or in deep water, and the climb still above it.
## The columns round-trip and start blank on both backends and the native store holds them
## on its field; the melee clears every target in the store as a fight begins, on both
## backends; the march writes where along the route each unit stands; the walk keeps its
## climb in the store, and stamina reads it there; and SkirmishUnit no longer declares them,
## nor its unread copies of its squad's order, home and wait. The native half is skipped
## where the Rust core isn't built (native/build.sh).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")

## The new columns, each with a value unlike its blank.
const SET := {
	UnitColumns.TARGET_ID: 7,
	UnitColumns.DISTANCE: 2.5,
	UnitColumns.FOOTHOLD: Vector2(3.5, 1.5),
	UnitColumns.ON_FACE: 1,
	UnitColumns.CLIMB_LEFT: 4.0,
}
const CLIMBER := [1.0, 0, 1, false, true]  # height, swimmer, climber, swims, climbs


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [[NativeKernels.GDSCRIPT, NativeKernels.unit_store(null)]]
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_unit_walk_columns: the Rust core isn't built - its half skipped")
	else:
		out.append([NativeKernels.RUST, NativeKernels.unit_store(field)])
	return out


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	return unit


func _squad(squad_id: int, members: Array, store: UnitStore) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(squad_id, "player", 1, 0.0, members.size(), typed, null, store)


func _melee() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.items = [WeaponDef.innate_weapon(1)]
	return unit_def


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
		if store.field() != null:  # the native store holds them on its field
			_assert_on_field(store.field(), slot)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		for name: StringName in SET:
			(
				assert_that(columns.value(slot, name))
				. override_failure_message("%s %s" % [said, name])
				. is_equal(UnitColumns.BLANKS[name])
			)


func _assert_on_field(field: Object, slot: int) -> void:
	assert_int(field.column_longs(UnitColumns.TARGET_ID)[slot]).is_equal(7)
	assert_float(field.column_reals(UnitColumns.DISTANCE)[slot]).is_equal(2.5)
	assert_vector(field.column_points(UnitColumns.FOOTHOLD)[slot]).is_equal(Vector2(3.5, 1.5))
	assert_int(field.column_ints(UnitColumns.ON_FACE)[slot]).is_equal(1)
	assert_float(field.column_reals(UnitColumns.CLIMB_LEFT)[slot]).is_equal(4.0)


## A fight begins with every living unit's target cleared, in the store, before any strikes:
## with no foe to strike, every target is 0 after the melee, on both backends.
func test_the_melee_clears_each_target_in_the_store_on_both_backends() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var said: String = backend[0]
		var fighter := _unit(1)
		var squad := _squad(1, [fighter], store)
		UnitHealth.put_one(squad, fighter, 10)
		store.columns.put(fighter.slot, UnitColumns.TARGET_ID, 42)
		FormationMelee.blows([squad], 10, 0, store.field())
		(
			assert_int(store.columns.value(fighter.slot, UnitColumns.TARGET_ID))
			. override_failure_message(said)
			. is_equal(0)
		)


## Each unit in its place stands along the route at its rank's distance, as the march
## writes it to the store, on both engines.
func test_the_march_writes_where_along_the_route_each_unit_stands() -> void:
	for engine in [NativeKernels.GDSCRIPT, NativeKernels.RUST]:
		NativeKernels.use(engine)
		var sim := FormationSimulation.new(9.0, 0.1)
		var placements := [[_melee(), Vector2i(0, 0)], [_melee(), Vector2i(1, 0)]]
		var squad := sim.spawn_squad(1, placements, "player", true)
		for tick in 3:
			sim.step()
		for unit in squad.units:
			var along: float = squad.store.columns.value(unit.slot, UnitColumns.DISTANCE)
			assert_float(along).override_failure_message(engine).is_equal_approx(
				squad.rank_distance(UnitPlace.rank(squad, unit)), 0.0001
			)
		NativeKernels.use(NativeKernels.GDSCRIPT)


## A climber stepping onto a cliff begins its climb in the store; on the face it takes no
## breather, as stamina reads ON_FACE there; once off, it does.
func test_the_walk_keeps_its_climb_in_the_store_and_stamina_reads_it() -> void:
	var ground := FormationTerrain.new(Vector2i(20, 20))
	ground.paint(Rect2i(10, 0, 10, 20), {"height": 16})  # 4 cells up at x 10
	var climber := _unit(1)
	climber.walker = TerrainWalker.new(CLIMBER[0], CLIMBER[1], CLIMBER[2], CLIMBER[3], CLIMBER[4])
	var squad := _squad(1, [climber], NativeKernels.unit_store(null))
	UnitHealth.put_one(squad, climber, 10)
	var columns := squad.store.columns
	columns.put(climber.slot, UnitColumns.FOOTHOLD, Vector2(9.5, 5.5))
	columns.put(climber.slot, UnitColumns.POSITION, Vector2(10.1, 5.5))

	FormationWalk.walk(squad, climber, Vector2(10.1, 5.5), [0.1, 8.0], ground)  # it stands

	assert_float(columns.value(climber.slot, UnitColumns.CLIMB_LEFT)).is_equal_approx(4.0, 0.0001)
	assert_int(columns.value(climber.slot, UnitColumns.ON_FACE)).is_equal(1)
	FormationStamina.step([squad], 1.0)
	assert_float(columns.value(climber.slot, UnitColumns.BREATHER)).is_equal(0.0)
	columns.put(climber.slot, UnitColumns.ON_FACE, 0)
	FormationStamina.step([squad], 1.0)
	assert_float(columns.value(climber.slot, UnitColumns.BREATHER)).is_equal(1.0)


func test_the_rest_of_the_per_tick_fields_are_the_store_s_alone() -> void:
	var unit := SkirmishUnit.new()
	for name in ["target_id", "distance", "foothold", "on_face", "climb_left"]:
		assert_bool(name in unit).override_failure_message(name).is_false()  # moved
	for name in ["order", "home_distance", "wait_ticks"]:  # the squad's, never read here
		assert_bool(name in unit).override_failure_message(name).is_false()
	assert_bool("carrying" in unit).is_true()  # event-only: its record's (Decision 147)
	assert_bool("playing_dead" in unit).is_true()
