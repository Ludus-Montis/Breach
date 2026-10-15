extends GdUnitTestSuite
## The scrum's walk on the native core (ScrumWalkField, Decision 129): a mid-fight scrum's
## units, planned and walked on the field, end exactly where GDScript walks them, looking
## where it has them look - at several moments, with ways pushed through the crowd and
## bodies laid underfoot, and whatever the list order. Skipped where the library isn't
## built (native/build.sh): the switch falls back to GDScript.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


func _field() -> Object:
	NativeKernels.use(NativeKernels.RUST)
	var field := NativeKernels.body_field()
	NativeKernels.use(NativeKernels.GDSCRIPT)
	if field == null:
		print("test_walk_field: the Rust core isn't built - skipped")
	return field


## A mid-fight scrum of a 40-a-side clash: the squads after `ticks` ticks under GDScript.
## `stirred`: bodies laid on the ground among the fighters, some a heavy weight, and one
## unit in five held back from seeking (out of the front band), so it walks to its place.
func _scrum(ticks: int, reversed: bool, stirred := false) -> Array:
	var sim := FormationBench.clash(40, 10, 2)
	for _tick in range(ticks):
		sim.step()
	var squads: Array = sim.squads().duplicate()
	if stirred:
		_lay_bodies(squads)
		for squad in squads:
			for unit in squad.living():
				unit.preferred_position = 1 if unit.id % 5 == 0 else unit.preferred_position
	if reversed:
		squads.reverse()
		for squad in squads:
			squad.units.reverse()
	return squads


## Lays a downed body a step ahead of every third fighter, one in four of them three
## cells broad (a weight that blocks a grem).
func _lay_bodies(squads: Array) -> void:
	var laid := []
	for squad in squads:
		for unit in squad.living():
			if unit.id % 3 != 0:
				continue
			var body := SkirmishUnit.new()
			body.id = 100000 + unit.id
			body.state = SkirmishUnit.State.DOWNED
			var size := 3 if unit.id % 4 == 0 else 1
			body.footprint_width = size
			body.footprint_depth = size
			body.slot = squad.store.enrol(body.id)
			var near := Vector2(sin(unit.id), cos(unit.id)) * 0.7
			UnitPlaces.put(squad, body, UnitPlaces.of(squad, unit) + near)
			laid.append([squad, body])
	for pair in laid:
		pair[0].units.append(pair[1])


## Plans the fighting squads' units once on `field` (null: GDScript), pushes the way of
## every other seeker on through its foe if `pushed`, then walks them; returns
## [every loose entry's [id, at, next, toward] by id, units seeking, units walking back].
func _walked(squads: Array, field: Object, pushed: bool) -> Array:
	var ctx := _ctx(squads, field)
	ScrumSeek.plan(ctx)
	if pushed:
		_push(squads, ctx)
	var fighting := squads.filter(func(s): return s.state == SkirmishSquad.State.FIGHTING)
	FormationScrum.walk(fighting, ctx)
	var counts := [0, 0]
	for squad in fighting:
		var columns: UnitColumns = squad.store.columns
		for unit in squad.loose.values():
			var seeks: bool = EntryKeys.goal(columns, unit.slot) != null
			counts[0] += 1 if seeks else 0
			counts[1] += 1 if not seeks and columns.value(unit.slot, UnitColumns.TOUCH) == 0 else 0
	return [_entries(squads)] + counts


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


## Every other seeker's next point moved on past its foe, so its way runs into bodies:
## in its entry, and in the plan's answer the field walks by.
func _push(squads: Array, ctx: Dictionary) -> void:
	var seek = ctx.get("seek_batch")
	var nexts: PackedVector2Array = seek.plan[3] if seek != null else PackedVector2Array()
	for squad in squads:
		var columns: UnitColumns = squad.store.columns
		for unit in squad.living():
			if not squad.loose.has(unit.id) or unit.id % 2 != 0:
				continue
			if EntryKeys.goal(columns, unit.slot) == null:
				continue
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var foe_at: Vector2 = columns.value(unit.slot, UnitColumns.FOE_AT)
			var next := at + (foe_at - at) * 2.5
			columns.put(unit.slot, UnitColumns.NEXT, next)
			if seek != null:
				nexts[seek.units.find(unit)] = next
	if seek != null:
		seek.plan[3] = nexts


## [[unit id, at, next, toward], ...] by id.
func _entries(squads: Array) -> Array:
	var out := []
	for squad in squads:
		var columns: UnitColumns = squad.store.columns
		for unit in squad.loose.values():
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var next: Vector2 = columns.value(unit.slot, UnitColumns.NEXT)
			out.append([unit.id, at, next, EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)])
	out.sort_custom(func(a, b): return a[0] < b[0])
	return out


## Every squad's loose entries and (last) the store's columns, which hold them.
func _saved(squads: Array) -> Array:
	return squads.map(func(s): return s.loose.duplicate()) + [UnitPlaces.saved(squads[0].store)]


func _restore(squads: Array, saved: Array) -> void:
	UnitPlaces.restore(squads[0].store, saved[-1])
	for index in range(squads.size()):
		squads[index].loose = saved[index].duplicate()


func _same(got: Array, want: Array) -> void:
	assert_array(Array(var_to_bytes(got))).is_equal(Array(var_to_bytes(want)))


func test_the_walk_on_the_field_steps_units_as_gdscript_does() -> void:
	var field := _field()
	if field == null:
		return
	for ticks in [95, 110, 130, 140]:  # moments while the scrum is under way
		var squads := _scrum(ticks, false)
		var saved := _saved(squads)
		var unwalked := _entries(squads)
		var reference := _walked(squads, null, false)
		assert_int(reference[1]).is_greater(0)  # units seek slots
		assert_array(reference[0]).is_not_equal(unwalked)  # and move
		_restore(squads, saved)
		_same(_walked(squads, field, false), reference)


func test_the_walk_on_the_field_steps_round_bodies_and_over_the_fallen_as_gdscript_does() -> void:
	var field := _field()
	if field == null:
		return
	for ticks in [110, 130]:
		var squads := _scrum(ticks, false, true)
		var saved := _saved(squads)
		var reference := _walked(squads, null, true)
		assert_int(reference[1]).is_greater(0)  # units seek slots
		assert_int(reference[2]).is_greater(0)  # and others walk back to their places
		_assert_obstructed(squads, saved)
		_restore(squads, saved)
		_same(_walked(squads, field, true), reference)


## The stirred scrum makes the walk's every rule count: some ways are stepped round a
## body, and some steps are slowed or stopped by the fallen.
func _assert_obstructed(squads: Array, saved: Array) -> void:
	_restore(squads, saved)
	var ctx := _ctx(squads, null)
	ScrumSeek.plan(ctx)
	_push(squads, ctx)
	var rounded := 0
	var slowed := 0
	var stopped := 0
	for squad in squads:
		var columns: UnitColumns = squad.store.columns
		for unit in squad.living():
			if not squad.loose.has(unit.id) or EntryKeys.goal(columns, unit.slot) == null:
				continue
			var at: Vector2 = SquadEntries.loose_at(squad, unit)
			var next: Vector2 = columns.value(unit.slot, UnitColumns.NEXT)
			var to := UnitSteer.toward(unit, at, next, ctx["bodies"], ctx["seed"], ctx["crowd"])
			rounded += 0 if to == next else 1
			var footing := FormationScrum._footing(unit, at, to, ctx)
			slowed += 1 if footing > 0.0 and footing < 1.0 else 0
			stopped += 1 if footing == 0.0 else 0
	assert_int(rounded).is_greater(0)
	assert_int(slowed).is_greater(0)
	assert_int(stopped).is_greater(0)


func test_the_walk_on_the_field_is_the_same_whatever_the_list_order() -> void:
	var field := _field()
	if field == null:
		return
	var reference := _walked(_scrum(120, false, true), null, true)
	_same(_walked(_scrum(120, false, true), field, true), reference)
	_same(_walked(_scrum(120, true, true), _field(), true), reference)
