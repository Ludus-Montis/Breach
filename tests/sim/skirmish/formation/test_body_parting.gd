extends GdUnitTestSuite
## The native core (NativeKernels, BodyParting, Decision 129): UnitBodies' body-parting
## pass run in Rust on a BodyField leaves every body bit for bit where the GDScript path
## does - framed, loose and fleeing bodies, friends and foes, bodies lying on each other, a
## field kept across ticks while units die and change squad, and a whole battle (the slot
## search on the field too) - and nothing turns on list order. Where the library isn't
## built (native/build.sh) the native cases are skipped: the switch falls back to GDScript.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")

const NATIVE := ["rust"]
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## The native engines built here (none on a checkout without native/bin).
func _built() -> Array:
	var out := NATIVE.filter(func(e): return NativeKernels.available(e))
	if out.is_empty():
		print("test_body_parting: no native kernel built - native cases skipped")
	return out


func _unit(unit_id: int, size: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.hp = 10
	unit.footprint_width = size
	unit.footprint_depth = size
	return unit


## Squads crowding one spot: a framed line with a loose friend and a router among them, a
## brute and grems lying exactly on each other, and a foe pressed in.
func _crowd(reversed: bool) -> Array:
	var units := [_unit(1), _unit(2, 2), _unit(3), _unit(4), _unit(5), _unit(6)]
	var points := [
		Vector2(4, 4),
		Vector2(4, 4),
		Vector2(4.2, 4),
		Vector2(4.6, 4.3),
		Vector2(5, 5),
		Vector2(3, 4)
	]
	var line := SkirmishSquad.new(
		1, "player", 1, 0.0, 4, [] as Array[SkirmishUnit], null, _places.store
	)
	for index in range(4):
		line.units.append(units[index])
		_places.stand(units[index], points[index])
	for index in [1, 2]:
		var at: Vector2 = points[index]
		SquadEntries.loosen(
			line, units[index], {"unit": units[index], "goal": null, "next": at}, at
		)
	SquadEntries.flee(line, units[3], {"along": 4.0}, Vector2(0.6, 4.3))
	var other := SkirmishSquad.new(
		2, "player", 1, 0.0, 1, [units[4]] as Array[SkirmishUnit], null, _places.store
	)
	_places.stand(units[4], points[4])
	var foes := SkirmishSquad.new(
		3, "the_kingdom", 1, 0.0, 1, [units[5]] as Array[SkirmishUnit], null, _places.store
	)
	SquadEntries.loosen(
		foes, units[5], {"unit": units[5], "goal": null, "next": points[5]}, points[5]
	)
	var squads := [line, other, foes]
	if reversed:
		squads.reverse()
		line.units.reverse()
	return squads


## Every squad's bodies, loose and fleeing entries, as bytes (exact, not approximate).
func _bytes(squads: Array) -> PackedByteArray:
	var state := []
	for squad in squads:
		for unit in squad.units:
			var loose: Variant = squad.loose.get(unit.id)
			var held := [unit.id, UnitPlaces.of(squad, unit), squad.fleeing.get(unit.id)]
			var at: Variant = null if loose == null else SquadEntries.loose_at(squad, unit)
			held.append(null if loose == null else [at, loose["next"], loose["goal"]])
			state.append(held)
	state.sort_custom(func(a, b): return a[0] < b[0])
	return var_to_bytes(state)


## The crowd parted for three ticks, each on a field of its own (the store moved onto it),
## or (`kept`) four on one field kept across them while a unit dies and another changes
## squad.
func _crowd_parted(engine: String, reversed: bool, kept := false) -> PackedByteArray:
	NativeKernels.use(engine)
	_places = UnitPlaces.new()  # each run its own store: no bit of another run's state
	var squads := _crowd(reversed)
	var field := NativeKernels.body_field() if kept else null
	if field != null:
		UnitPlaces.onto(field, squads)
	for tick in range(4 if kept else 3):
		if kept and tick == 2:
			_churn(squads)
		if not kept and NativeKernels.body_field() != null:
			field = NativeKernels.body_field()
			UnitPlaces.onto(field, squads)
		UnitBodies.step(squads, 9 + tick, field)
	return _bytes(squads)


## Unit 3 dies; unit 5 leaves its squad for the line (squad 1).
func _churn(squads: Array) -> void:
	var by_id := {}
	for squad in squads:
		by_id[squad.id] = squad
		for unit in squad.living():
			if unit.id == 3:
				unit.state = SkirmishUnit.State.DEAD
	var mover: SkirmishUnit = by_id[2].units[0]
	by_id[2].units.erase(mover)
	by_id[1].units.append(mover)


func test_a_field_kept_across_ticks_parts_as_gdscript_does_as_units_die_and_move() -> void:
	var reference := _crowd_parted(NativeKernels.GDSCRIPT, false, true)
	for engine in _built():
		assert_array(Array(_crowd_parted(engine, false, true))).is_equal(Array(reference))
		assert_array(Array(_crowd_parted(engine, true, true))).is_equal(Array(reference))


func test_the_gdscript_path_is_the_default_and_an_unknown_engine_is_not_available() -> void:
	assert_bool(NativeKernels.available(NativeKernels.GDSCRIPT)).is_true()
	assert_bool(NativeKernels.available("fortran")).is_false()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	assert_object(NativeKernels.body_field()).is_null()


func test_a_native_kernel_parts_a_crowd_bit_for_bit_as_gdscript_does() -> void:
	var reference := _crowd_parted(NativeKernels.GDSCRIPT, false)
	for engine in _built():
		var parted := _crowd_parted(engine, false)
		assert_object(NativeKernels.body_field()).is_not_null()  # the core ran
		assert_array(Array(parted)).is_equal(Array(reference))


func test_a_native_kernel_parts_the_same_whatever_the_list_order() -> void:
	for engine in _built():
		var forward := _crowd_parted(engine, false)
		assert_array(Array(_crowd_parted(engine, true))).is_equal(Array(forward))


func test_a_native_kernel_plays_a_battle_bit_for_bit_as_gdscript_does() -> void:
	var reference := _battle(NativeKernels.GDSCRIPT)
	for engine in _built():
		assert_array(_battle(engine)).is_equal(reference)


## Each tick's bytes of a 40-a-side clash, 10 wide, from just before contact (tick 90) on.
func _battle(engine: String) -> Array:
	NativeKernels.use(engine)
	var sim := FormationBench.clash(40, 10, 1)
	var out := []
	for tick in range(150):
		sim.step()
		if tick >= 80:
			out.append(_bytes(sim.squads()))
	return out
