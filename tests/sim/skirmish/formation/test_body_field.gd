extends GdUnitTestSuite
## The native core's BodyField (Decision 129, NativeKernels): Godot's hash() reproduced bit
## for bit, a roster kept across ticks (units enrolled once, the dead dropped, a unit that
## changes squad moved with it), and the scrum's slot search on the field choosing exactly
## what ScrumSeek's GDScript chooses - whatever the list order. Skipped where the library
## isn't built (native/build.sh): the switch falls back to GDScript.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


func _field() -> Object:
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_body_field: the Rust core isn't built - skipped")
	return field


func test_the_core_hashes_as_godot_does() -> void:
	var field := _field()
	if field == null:
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 129
	for trial in range(2000):
		var wide := rng.randi() * (1 << 31) + rng.randi()  # 62 bits, as the draws are
		var shapes := [
			[rng.randi_range(-9, 1 << 30), rng.randi_range(0, 9000), rng.randi_range(1, 5000)],
			[rng.randi(), rng.randi_range(1, 5000), rng.randi_range(1, 5000), trial % 9, "slot"],
			[rng.randi(), wide, wide - trial, "part"],
			[rng.randi(), "squad", rng.randi_range(1, 99), trial % 2],
			[-wide, 0, 1],
		]
		for keys in shapes:
			assert_int(field.godot_hash(keys)).is_equal(hash(keys))


func _unit(unit_id: int, at: Vector2) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.health(unit, 10)
	_places.stand(unit, at)
	return unit


func test_the_roster_is_kept_across_syncs_and_changes_only_where_units_do() -> void:
	var field := _field()
	if field == null:
		return
	_places = UnitPlaces.new(field)  # the store on the field, as a battle's is
	var units: Array[SkirmishUnit] = [_unit(1, Vector2(1, 1)), _unit(2, Vector2(2, 1))]
	var one := SkirmishSquad.new(1, "player", 1, 0.0, 2, units, null, _places.store)
	var others: Array[SkirmishUnit] = [_unit(3, Vector2(5, 5))]
	var other := SkirmishSquad.new(2, "player", 1, 0.0, 1, others, null, _places.store)
	BodyFieldSync.bodies(field, [one, other], 7)
	assert_int(field.count()).is_equal(3)
	one.units[0].state = SkirmishUnit.State.DEAD
	var moved := one.units[1]
	one.units.erase(moved)
	other.units.append(moved)
	_places.stand(moved, Vector2(6, 5))
	var synced := BodyFieldSync.bodies(field, [one, other], 7)
	assert_int(field.count()).is_equal(2)  # the dead dropped, the mover kept
	assert_array(synced[0]).is_equal([other.units[0], moved])
	assert_array(Array(field.points())).is_equal([Vector2(5, 5), Vector2(6, 5)])


## A sync names only the roster (spec 35 step 4e): how each unit stands (loose or not) is
## read from the unit store's flag bits on the field, so a loose unit stands where it
## stands loose and one in its frame at its position.
func test_a_sync_reads_how_each_unit_stands_from_the_store() -> void:
	var field := _field()
	if field == null:
		return
	_places = UnitPlaces.new(field)
	var units: Array[SkirmishUnit] = [_unit(1, Vector2(1, 1)), _unit(2, Vector2(2, 1))]
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 2, units, null, _places.store)
	SquadEntries.loosen(squad, units[1], Vector2(4, 3))
	var ids := PackedInt64Array([1, 2])
	var squad_ids := PackedInt64Array([1])
	var factions := PackedStringArray(["player"])
	var counts := PackedInt32Array([2])
	var unknown: PackedInt32Array = field.sync(
		7, squad_ids, factions, PackedByteArray([0]), counts, ids, BodyFieldSync.SCRUM, []
	)
	assert_array(Array(unknown)).is_equal([0, 1])
	assert_array(Array(field.points())).is_equal([Vector2(1, 1), Vector2(4, 3)])


## A mid-fight scrum of a 40-a-side clash: the squads after `ticks` ticks under GDScript.
func _scrum(ticks: int, reversed: bool) -> Array:
	var sim := FormationBench.clash(40, 10, 2)
	for _tick in range(ticks):
		sim.step()
	var squads: Array = sim.squads().duplicate()
	if reversed:
		squads.reverse()
		for squad in squads:
			squad.units.reverse()
	return squads


func _ctx(squads: Array, field: Object) -> Dictionary:
	if field != null:
		UnitPlaces.onto(field, squads)  # its passes read the store's columns on the field
	var ctx := {"squads": squads, "tick": 7, "seed": 2, "terrain": null, "active": {}}
	ctx["field"] = field
	ctx["bodies"] = ScrumSeek.bodies(squads)
	ctx["crowd"] = BodyGrid.of_bodies(ctx["bodies"])
	return ctx


## Every unit's seeking state after one plan, by unit id, with the squads made active.
func _planned(squads: Array, field: Object) -> Array:
	var ctx := _ctx(squads, field)
	ScrumSeek.plan(ctx)
	var out := []
	for squad in squads:
		for unit_id in squad.loose:
			var entry := EntryKeys.loose_entry(
				squad.store.columns, squad.loose[unit_id], Vector2.ZERO
			)
			var held := [entry["goal"], entry["next"], entry.get("foe_at"), entry.get("touch")]
			out.append([unit_id, held])
	out.sort_custom(func(a, b): return a[0] < b[0])
	var active: Array = ctx["active"].keys()
	active.sort()
	return [out, active]


## The squads' loose entries and the store's columns, which hold them (spec 35).
func _saved(squads: Array) -> Array:
	return (
		squads.map(func(squad): return squad.loose.duplicate())
		+ [UnitPlaces.saved(squads[0].store)]
	)


func _restore(squads: Array, saved: Array) -> void:
	for index in range(squads.size()):
		squads[index].loose = saved[index].duplicate()
	UnitPlaces.restore(squads[0].store, saved[-1])


func test_the_slot_search_on_the_field_seeks_as_gdscript_does() -> void:
	var field := _field()
	if field == null:
		return
	# Moments while the scrum is under way (with DetMath it ends by tick 150, not 165)
	for ticks in [95, 110, 130, 140]:
		var squads := _scrum(ticks, false)
		var saved := _saved(squads)
		var reference := _planned(squads, null)
		var seeking: Array = reference[0].filter(func(e): return e[1][0] != null)
		assert_bool(seeking.is_empty()).is_false()  # the scrum is under way
		_restore(squads, saved)
		var planned := var_to_bytes(_planned(squads, field))
		assert_array(Array(planned)).is_equal(Array(var_to_bytes(reference)))


func test_a_synced_unit_s_record_is_enrolled_on_the_field_once() -> void:
	var field := _field()
	if field == null:
		return
	var squads := _scrum(95, false)
	UnitPlaces.onto(field, squads)
	BodyFieldSync.scrum(field, squads, 2)
	var checked := 0
	for squad in squads:
		for unit in squad.living():
			var record := [
				float(unit.preferred_position),
				float(unit.footprint_width),
				float(unit.footprint_depth),
				unit.turn_rate,
				unit.backward_pace,
			]
			assert_array(Array(field.record(unit.slot))).is_equal(record)
			checked += 1
	assert_int(checked).is_greater(0)


func test_the_slot_search_on_the_field_is_the_same_whatever_the_list_order() -> void:
	var field := _field()
	if field == null:
		return
	var forward := _planned(_scrum(120, false), field)
	var backward := _planned(_scrum(120, true), _field())
	assert_array(Array(var_to_bytes(backward))).is_equal(Array(var_to_bytes(forward)))
