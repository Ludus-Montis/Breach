extends GdUnitTestSuite
## The squad store (specs/39-simulation-in-rust.md step 1): a squad's state lives in its
## store's squad columns (SquadColumns), and SkirmishSquad reads and writes it there, on the
## battle's BodyField (SquadColumnsNative): a new squad takes the next slot and starts as before,
## each field is its column, its members are its units' slots in its order, and a squad
## carried to another store keeps its state. The crate's snapshot reads back the squads it
## wrote.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadColumns = preload("res://sim/skirmish/formation/squad_columns.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")

## Each field, its column and a value to write that no squad starts with.
const FIELDS := [
	["direction", SquadColumns.DIRECTION, -1],
	["width", SquadColumns.WIDTH, 7],
	["order", SquadColumns.ORDER, SkirmishUnit.Order.RETREAT],
	["state", SquadColumns.STATE, SkirmishSquad.State.ROUTING],
	["fought", SquadColumns.FOUGHT, true],
	["patched_count", SquadColumns.PATCHED_COUNT, 3],
	["stall_ticks", SquadColumns.STALL_TICKS, 5],
	["manoeuvre", SquadColumns.MANOEUVRE, 1],
	["morale", SquadColumns.MORALE, 42],
	["rally_ticks", SquadColumns.RALLY_TICKS, 9],
	["blocked", SquadColumns.BLOCKED, true],
	["painted_width", SquadColumns.PAINTED_WIDTH, 4],
	["narrow_ticks", SquadColumns.NARROW_TICKS, 6],
	["wait_ticks", SquadColumns.WAIT_TICKS, 12],
	["reforming", SquadColumns.REFORMING, true],
	["combat_width", SquadColumns.COMBAT_WIDTH, 30],
	["engaged_with", SquadColumns.ENGAGED_WITH, 1 << 40],
	["fight_since", SquadColumns.FIGHT_SINCE, 77],
	["taking_since", SquadColumns.TAKING_SINCE, 78],
	["took_until", SquadColumns.TOOK_UNTIL, 79],
	["home_distance", SquadColumns.HOME_DISTANCE, 12.5],
	["heading", SquadColumns.HEADING, 0.1 + 0.2],
	["looked_at", SquadColumns.LOOKED_AT, 33.25],
	["painted_shift", SquadColumns.PAINTED_SHIFT, -1.5],
	["centre_shift", SquadColumns.CENTRE_SHIFT, 2.5],
]


## Each engine that runs here: [name, a new unit store on it].
func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	return out


func _units(first_id: int, count: int) -> Array[SkirmishUnit]:
	var out: Array[SkirmishUnit] = []
	for n in count:
		var unit := SkirmishUnit.new()
		unit.id = first_id + n
		out.append(unit)
	return out


func _squad(store: UnitStore, squad_id: int, members: Array[SkirmishUnit]) -> SkirmishSquad:
	return SkirmishSquad.new(
		squad_id, "player", 1, 2.0, 3, members, FormationCommand.new(2.0), store
	)


func test_a_new_squad_takes_the_next_slot_and_starts_as_before() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var first := _squad(store, 7, [])
		var second := _squad(store, 3, [])
		var where: String = backend[0]
		assert_array([first.slot, second.slot]).override_failure_message(where).is_equal([0, 1])
		assert_int(store.squads.count()).is_equal(2)
		assert_float(second.heading).is_equal(90.0)  # east: direction +1
		assert_int(second.order).is_equal(SkirmishUnit.Order.ADVANCE)
		assert_int(second.state).is_equal(SkirmishSquad.State.MOVING)
		assert_int(second.fight_since).is_equal(-1)
		assert_int(second.took_until).is_equal(-1)
		assert_float(second.looked_at).is_equal(-INF)
		assert_int(second.manoeuvre).is_equal(4)
		assert_int(second.morale).is_equal(-1)
		assert_bool(second.fought).is_false()
		assert_float(second.front_distance).is_equal(2.0)
		assert_float(second.home_distance).is_equal(2.0)
		assert_int(second.width).is_equal(3)
		assert_int(second.direction).is_equal(1)


func test_each_field_is_its_column_in_the_squad_store() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		_squad(store, 1, [])
		var squad := _squad(store, 2, [])
		for field in FIELDS:
			squad.set(field[0], field[2])
			var stored = store.squads.value(squad.slot, field[1])
			var where := "%s %s" % [backend[0], field[0]]
			assert_that(squad.get(field[0])).override_failure_message(where).is_equal(field[2])
			assert_that(stored).override_failure_message(where).is_equal(
				int(field[2]) if field[2] is bool else field[2]
			)
		assert_int(store.squads.value(0, SquadColumns.WIDTH)).is_equal(3)  # its own slot only
		squad.front_distance = 4.0  # moves its front along its route with it
		assert_float(store.squads.value(squad.slot, SquadColumns.FRONT_DISTANCE)).is_equal(4.0)
		assert_that(store.squads.value(squad.slot, SquadColumns.POSITION)).is_equal(squad.position)


func test_its_members_are_its_units_slots_in_its_order() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var units := _units(10, 4)
		var squad := _squad(store, 1, units)
		var slots := PackedInt32Array(units.map(func(u): return u.slot))
		var where: String = backend[0]
		squad.living()  # its lists rebuilt: its members written
		assert_array(store.squads.members(squad.slot)).override_failure_message(where).is_equal(
			slots
		)
		squad.units.erase(units[1])
		UnitState.regather(squad)
		squad.living()
		slots.remove_at(1)
		assert_array(store.squads.members(squad.slot)).is_equal(slots)


func test_a_squad_carried_to_another_store_keeps_its_state() -> void:
	var others := _backends()  # a second store on each engine, on a field of its own
	for index in others.size():
		var backend: Array = _backends()[index]
		var squad := _squad(backend[1], 1, [])
		squad.morale = 55
		squad.heading = 180.0
		var other: UnitStore = others[index][1]
		_squad(other, 9, [])  # its slot 0 taken: the squad's slot changes
		squad.store = other
		assert_int(squad.slot).override_failure_message(backend[0]).is_equal(1)
		assert_int(squad.morale).is_equal(55)
		assert_float(squad.heading).is_equal(180.0)


func test_the_crates_snapshot_reads_back_the_squads_it_wrote() -> void:
	var backends := _backends()
	if backends.size() < 2:
		return
	var store: UnitStore = backends[1][1]
	var squad := _squad(store, 4, _units(1, 3))
	squad.state = SkirmishSquad.State.FIGHTING
	squad.looked_at = -INF
	var bytes: PackedByteArray = store.field().snapshot(40, 3)
	var field := NativeKernels.body_field()
	assert_array(field.restore(bytes)).is_equal(PackedInt64Array([40, 3]))
	var back := SquadColumnsNative.new(field)
	assert_int(back.count()).is_equal(1)
	assert_int(back.value(0, SquadColumns.STATE)).is_equal(SkirmishSquad.State.FIGHTING)
	assert_float(back.value(0, SquadColumns.LOOKED_AT)).is_equal(-INF)
	assert_array(back.members(0)).is_equal(store.squads.members(squad.slot))
	assert_array(field.restore(PackedByteArray([1, 2, 3]))).is_empty()
