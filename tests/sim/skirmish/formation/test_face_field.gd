extends GdUnitTestSuite
## The scrum's facing on the native core (ScrumFaceField, Decision 129): DetMath's sin,
## cos, asin, acos and atan2 give the GDScript's bits in Rust, and a mid-fight scrum's
## units, planned, walked and turned on the field, end on exactly the bearings GDScript
## turns them to - whatever the list order. Skipped where the library isn't built
## (native/build.sh): the switch falls back to GDScript.

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
		for entry in squad.loose.values():
			counts[0] += 1 if entry.get("touch", false) else 0
			counts[1] += 0 if entry["goal"] == null else 1
	return [_bearings(squads)] + counts


func _ctx(squads: Array, field: Object) -> Dictionary:
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
			out.append([unit.id, unit.bearing])
	out.sort_custom(func(a, b): return a[0] < b[0])
	return out


func _saved(squads: Array) -> Array:
	var out := []
	for squad in squads:
		out.append([squad.loose.duplicate(true), squad.units.map(func(u): return u.bearing)])
	return out


func _restore(squads: Array, saved: Array) -> void:
	for index in range(squads.size()):
		squads[index].loose = saved[index][0].duplicate(true)
		for unit_index in range(squads[index].units.size()):
			squads[index].units[unit_index].bearing = saved[index][1][unit_index]


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
		var synced := BodyFieldSync.scrum(field, squads, ctx["seed"])
		ctx["field"] = field
		ctx["seek_batch"] = ScrumSeekField._gather(synced[0], synced[1], squads)
		FormationScrum.face(fighting, ctx)
		assert_array(Array(var_to_bytes(_bearings(squads)))).is_equal(
			Array(var_to_bytes(reference))
		)


## Turns every unit to a bearing of its own, and points every look its walk chose
## (`toward`) somewhere round it - a third of the seekers keeping to their places
## instead - so every branch of the turn is taken.
func _stirred(squads: Array) -> void:
	for squad in squads:
		for unit in squad.units:
			unit.bearing = fposmod(unit.id * 47.3, 360.0)
		for entry in squad.loose.values():
			var id: int = entry["unit"].id
			if entry.get("toward") != null:
				entry["toward"] = (
					SquadEntries.loose_at(squad, entry["unit"])
					+ Vector2(sin(id * 1.7), cos(id * 1.7)) * 2.0
				)
			if entry["goal"] != null and id % 3 == 0:
				entry["goal"] = null


func _bearings_of(saved: Array, squads: Array) -> Array:
	_restore(squads, saved)
	return _bearings(squads)
