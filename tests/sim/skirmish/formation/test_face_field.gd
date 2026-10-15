extends GdUnitTestSuite
## The scrum's facing on the native core (ScrumFaceField, Decision 129): DetMath's sin,
## cos, asin, acos and atan2 give the GDScript's bits in Rust, and a mid-fight scrum's
## units, planned, walked and turned on the field, end on exactly the bearings GDScript
## turns them to - whatever the list order. Skipped where the library isn't built
## (native/build.sh): the switch falls back to GDScript.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const BodyFieldSync = preload("res://sim/skirmish/formation/body_field_sync.gd")
const ScrumSeekField = preload("res://sim/skirmish/formation/scrum_seek_field.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")

const COUNT := 20000


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


func _field() -> Object:
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_face_field: the Rust core isn't built - skipped")
	return field


func test_det_math_in_rust_gives_the_gdscript_s_bits() -> void:
	if _field() == null:
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 130
	var xs := PackedFloat64Array([0.0, 1.0, -1.0, 0.5, -0.5, 1e-9, 1.5, -2.0, 1e30, 0.4375])
	var ys := PackedFloat64Array([1.0, 0.0, -1.0, 0.0, 2.0, -1e-9, 1.5, 3.0, -1.0, 1.1875])
	for i in range(COUNT):
		var scale := pow(10.0, rng.randi_range(-8, 4))  # near the axes, and far out
		xs.append(rng.randf_range(-1.0, 1.0) * (scale if i % 2 else 1.0))
		ys.append(rng.randf_range(-1.0, 1.0) * scale)
	var ours := {
		"sin": func(x, _y): return DetMath.sin(x * 400.0),
		"cos": func(x, _y): return DetMath.cos(x * 400.0),
		"asin": func(x, _y): return DetMath.asin(x),
		"acos": func(x, _y): return DetMath.acos(x),
		"atan2": func(x, y): return DetMath.atan2(x, y),
	}
	for op in ours:
		var a := xs.duplicate()
		if op in ["sin", "cos"]:
			for i in range(a.size()):
				a[i] = a[i] * 400.0
		var got: PackedFloat64Array = ClassDB.class_call_static("BodyField", "det_math", op, a, ys)
		var want := PackedFloat64Array()
		for i in range(xs.size()):
			want.append(ours[op].call(xs[i], ys[i]))
		assert_array(Array(got.to_byte_array())).override_failure_message(op).is_equal(
			Array(want.to_byte_array())
		)


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


## Plans, walks and turns the fighting squads' units once (FormationScrum._seek) on
## `field` (null: GDScript); returns [every unit's bearing by id, units touching a foe,
## units seeking].
func _faced(squads: Array, field: Object) -> Array:
	var ctx := _ctx(squads, field)
	FormationScrum._seek(ctx)
	var counts := [0, 0]
	for squad in squads:
		var columns: UnitColumns = squad.store.columns
		for unit in squad.loose.values():
			counts[0] += 1 if columns.value(unit.slot, UnitColumns.TOUCH) == 1 else 0
			counts[1] += 0 if EntryKeys.goal(columns, unit.slot) == null else 1
	return [_bearings(squads)] + counts


func _ctx(squads: Array, field: Object) -> Dictionary:
	if field != null:
		UnitPlaces.onto(field, squads)  # its passes read the store's columns on the field
	var ctx := {"squads": squads, "tick": 7, "seed": 2, "terrain": null, "active": {}}
	ctx["field"] = field
	ctx["seconds"] = 0.1
	ctx["pace"] = 0.8
	ctx["bodies"] = ScrumSeek.bodies(squads)
	ctx["crowd"] = BodyGrid.of_bodies(ctx["bodies"])
	ctx["lying"] = GroundBodies.ground(squads)
	return ctx


## [[unit id, bearing], ...] by id.
func _bearings(squads: Array) -> Array:
	var out := []
	for squad in squads:
		for unit in squad.units:
			out.append([unit.id, UnitPlaces.bearing_of(squad, unit)])
	out.sort_custom(func(a, b): return a[0] < b[0])
	return out


## Every squad's loose entries, and (last) the store's columns, bearings among them.
func _saved(squads: Array) -> Array:
	var out := []
	for squad in squads:
		out.append([squad.loose.duplicate()])
	out.append(UnitPlaces.saved(squads[0].store))
	return out


func _restore(squads: Array, saved: Array) -> void:
	UnitPlaces.restore(squads[0].store, saved[-1])
	for index in range(squads.size()):
		squads[index].loose = saved[index][0].duplicate()


func test_the_facing_on_the_field_turns_units_as_gdscript_does() -> void:
	var field := _field()
	if field == null:
		return
	for ticks in [95, 110, 130, 140]:  # moments while the scrum is under way
		var squads := _scrum(ticks, false)
		var saved := _saved(squads)
		var unturned := _bearings(squads)
		var reference := _faced(squads, null)
		assert_int(reference[1]).is_greater(0)  # units touch foes
		assert_int(reference[2]).is_greater(0)  # and others seek them
		assert_array(reference[0]).is_not_equal(unturned)  # and they turn
		_restore(squads, saved)
		assert_array(_bearings(squads)).is_equal(unturned)
		var turned := _faced(squads, field)
		assert_array(Array(var_to_bytes(turned))).is_equal(Array(var_to_bytes(reference)))


func test_the_facing_on_the_field_is_the_same_whatever_the_list_order() -> void:
	var field := _field()
	if field == null:
		return
	var reference := _faced(_scrum(120, false), null)
	var forward := _faced(_scrum(120, false), field)
	var backward := _faced(_scrum(120, true), _field())
	assert_array(Array(var_to_bytes(forward))).is_equal(Array(var_to_bytes(reference)))
	assert_array(Array(var_to_bytes(backward))).is_equal(Array(var_to_bytes(reference)))


func test_the_facing_on_the_field_turns_any_bearing_to_any_look_as_gdscript_does() -> void:
	var field := _field()
	if field == null:
		return
	for ticks in [110, 130]:
		var squads := _scrum(ticks, false)
		var ctx := _ctx(squads, null)
		FormationScrum._seek(ctx)  # the entries a walk leaves
		_stirred(squads)  # every bearing, and the walk's looks all round
		var saved := _saved(squads)
		var fighting := squads.filter(func(s): return s.state == SkirmishSquad.State.FIGHTING)
		FormationScrum.face(fighting, ctx)
		var reference := _bearings(squads)
		assert_array(reference).is_not_equal(_bearings_of(saved, squads))
		_restore(squads, saved)
		UnitPlaces.onto(field, squads)
		var synced := BodyFieldSync.scrum(field, squads, ctx["seed"])
		ctx["field"] = field
		ctx["seek_batch"] = ScrumSeekField.gathered(synced[0], synced[1], squads)
		FormationScrum.face(fighting, ctx)
		assert_array(Array(var_to_bytes(_bearings(squads)))).is_equal(
			Array(var_to_bytes(reference))
		)


## The facing on the field reads where each unit stands, faces and makes for from the unit
## store's columns and writes its bearing there (spec 35 step 4d): given only the squads,
## their foes, the units and their speeds, it turns them as GDScript does.
func test_the_facing_on_the_field_reads_and_writes_the_store_alone() -> void:
	var field := _field()
	if field == null:
		return
	var squads := _scrum(130, false)
	var ctx := _ctx(squads, null)
	FormationScrum._seek(ctx)  # the entries a walk leaves
	var saved := _saved(squads)
	var fighting := squads.filter(func(s): return s.state == SkirmishSquad.State.FIGHTING)
	FormationScrum.face(fighting, ctx)
	var reference := _bearings(squads)
	_restore(squads, saved)
	UnitPlaces.onto(field, squads)
	var synced := BodyFieldSync.scrum(field, squads, ctx["seed"])
	var seek := ScrumSeekField.gathered(synced[0], synced[1], squads)
	var tuning := BattleTuning.current()
	var front := DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001
	var floats := PackedFloat64Array(
		[tuning.reach_contact, front, ctx["seconds"], ctx["pace"], tuning.scrum_crowding]
	)
	var bit: int = EntryKeys.VALUED[UnitColumns.TOWARD]
	field.face(floats, seek.squads, seek.foe_ids, seek.members, seek.speeds, bit)
	assert_array(Array(var_to_bytes(_bearings(squads)))).is_equal(Array(var_to_bytes(reference)))


## Turns every unit to a bearing of its own, and points every look its walk chose
## (`toward`) somewhere round it - a third of the seekers keeping to their places
## instead - so every branch of the turn is taken.
func _stirred(squads: Array) -> void:
	for squad in squads:
		for unit in squad.units:
			UnitPlaces.turn(squad, unit, fposmod(unit.id * 47.3, 360.0))
		var columns: UnitColumns = squad.store.columns
		for unit in squad.loose.values():
			var id: int = unit.id
			if EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD) != null:
				var round_it := Vector2(sin(id * 1.7), cos(id * 1.7)) * 2.0
				var toward: Vector2 = SquadEntries.loose_at(squad, unit) + round_it
				EntryKeys.keep(columns, unit.slot, UnitColumns.TOWARD, toward)
			if EntryKeys.goal(columns, unit.slot) != null and id % 3 == 0:
				EntryKeys.set_goal(columns, unit.slot, null)


func _bearings_of(saved: Array, squads: Array) -> Array:
	_restore(squads, saved)
	return _bearings(squads)
