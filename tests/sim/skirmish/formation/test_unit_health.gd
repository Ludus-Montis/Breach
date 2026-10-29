extends GdUnitTestSuite
## Each unit's hp, the ticks till its next blow and the hp it may still regenerate in the
## unit store (specs/35-unit-store.md step 5b, Decision 145): the columns round-trip and
## start blank and the native store holds them on its field; every hp
## write goes through UnitHealth, which puts a unit an hp write takes from above 0 to 0 or
## below on the store's fell queue; and FormationDeaths.bury drains that queue, taking the
## fallen in their squads' order then their own, each with the squad it is in now.
## SkirmishUnit no longer declares the three.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	return out


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10
	unit.attributes = {"constitution": 6}
	return unit


func _squad(squad_id: int, members: Array, store: UnitStore) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(squad_id, "player", 1, 0.0, members.size(), typed, null, store)


func test_the_columns_round_trip_and_a_new_slot_starts_blank() -> void:
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var said: String = backend[0]
		var slot := store.enrol(5)
		assert_int(columns.value(slot, UnitColumns.HP)).override_failure_message(said).is_equal(0)
		columns.put(slot, UnitColumns.HP, 12)
		columns.put(slot, UnitColumns.ATTACK_COOLDOWN, 3)
		columns.put(slot, UnitColumns.REGENERATION_LEFT, 2.5)
		assert_int(columns.value(slot, UnitColumns.HP)).is_equal(12)
		assert_int(columns.column(UnitColumns.ATTACK_COOLDOWN)[slot]).is_equal(3)
		assert_float(columns.value(slot, UnitColumns.REGENERATION_LEFT)).is_equal(2.5)
		var field: Object = store.field()
		if field != null:  # the native store holds them on its field
			assert_int(field.column_ints(UnitColumns.HP)[slot]).is_equal(12)
			assert_int(field.column_ints(UnitColumns.ATTACK_COOLDOWN)[slot]).is_equal(3)
			assert_float(field.column_reals(UnitColumns.REGENERATION_LEFT)[slot]).is_equal(2.5)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(said).is_equal(slot)
		assert_int(columns.value(slot, UnitColumns.HP)).override_failure_message(said).is_equal(0)
		assert_int(columns.value(slot, UnitColumns.ATTACK_COOLDOWN)).is_equal(0)
		assert_float(columns.value(slot, UnitColumns.REGENERATION_LEFT)).is_equal(0.0)


## A unit falls when an hp write takes it from above 0 to 0 or below, and only then: a hurt
## that leaves it standing, one on a unit already at 0 or below, a heal and a put that
## doesn't cross the line push nothing.
func test_a_write_that_takes_a_unit_to_0_or_below_puts_it_on_the_fell_queue() -> void:
	for backend in _backends():
		var holder := {"store": backend[1]}  # anything holding the store
		var said: String = backend[0]
		var one := _unit(1)
		var two := _unit(2)
		var three := _unit(3)
		for unit in [one, two, three]:
			unit.slot = holder.store.enrol(unit.id)
			UnitHealth.put_one(holder, unit, 10)
		UnitHealth.hurt_one(holder, two, 4)
		assert_int(UnitHealth.of_one(holder, two)).is_equal(6)
		UnitHealth.hurt_one(holder, two, 6)  # to 0: falls
		UnitHealth.hurt_one(holder, two, 3)  # already down: no second fall
		UnitHealth.put_one(holder, one, -1)  # set past the line: falls
		UnitHealth.put_one(holder, three, UnitHealth.of_one(holder, three) + 5)
		UnitHealth.put_one(holder, three, 4)
		assert_int(UnitHealth.of_one(holder, two)).is_equal(-3)
		var fallen: PackedInt64Array = holder.store.fallen.take()
		assert_array(Array(fallen)).override_failure_message(said).is_equal([2, 1])
		assert_array(Array(holder.store.fallen.take())).override_failure_message(said).is_empty()
		# back above 0, then down again: a second fall
		UnitHealth.put_one(holder, two, UnitHealth.of_one(holder, two) + 5)
		UnitHealth.hurt_one(holder, two, 2)
		assert_array(Array(holder.store.fallen.take())).is_equal([2])


## The fight's hurts land at once in the crate (spec 39 step 8b2), in the order given: each
## answers its unit's hp after it, and the fallen go on the fell queue in the order they fell,
## once each.
func test_the_fight_s_hurts_land_in_order_and_fell_the_fallen_once() -> void:
	var holder := {"store": NativeKernels.unit_store(null)}
	var one := _unit(1)
	var two := _unit(2)
	for unit in [one, two]:
		unit.slot = holder.store.enrol(unit.id)
		UnitHealth.put_one(holder, unit, 5)
	var left := UnitHealth.hurt_landed(holder, [[two, 5], [one, 2], [two, 1], [one, 4]])
	assert_array(Array(left)).is_equal([0, 3, -1, -1])
	assert_array(Array(holder.store.fallen.take())).is_equal([2, 1])
	assert_array(Array(UnitHealth.hurt_landed(holder, []))).is_empty()


## A pass over many units holds hp in its view (UnitView, Decision 146) and writes it in
## place; release commits once, and a write there that crosses the line still pushes the unit.
func test_a_held_view_of_hp_writes_in_place_and_commits_on_release() -> void:
	for backend in _backends():
		var holder := {"store": backend[1]}
		var said: String = backend[0]
		var one := _unit(1)
		var two := _unit(2)
		for unit in [one, two]:
			unit.slot = holder.store.enrol(unit.id)
			UnitHealth.put_one(holder, unit, 10)
		var view := UnitView.hold([holder], [UnitColumns.HP])
		UnitHealth.hurt(view, one, 3)
		UnitHealth.hurt(view, two, 12)
		assert_int(UnitHealth.of(view, one)).is_equal(7)
		assert_int(UnitHealth.of_one(holder, one)).is_equal(7)  # every reader in the pass
		UnitView.release(view)
		assert_int(UnitHealth.of_one(holder, one)).override_failure_message(said).is_equal(7)
		assert_int(UnitHealth.of_one(holder, two)).is_equal(-2)
		assert_array(Array(holder.store.fallen.take())).override_failure_message(said).is_equal([2])


## Deaths drains the fell queue (Decision 145): the fallen in their squads' order then their
## own, never the order they fell; one healed above 0 or no longer living is skipped; one
## that moved squads after it fell is buried with the squad it is in now; and a living unit
## at 0 hp no write took there is no fallen unit.
func test_bury_takes_the_fallen_from_the_queue_in_the_lists_order() -> void:
	for backend in _backends():
		var places := UnitPlaces.new(backend[1].field())
		var said: String = backend[0]
		var units := []
		for unit_id in range(1, 8):
			units.append(_unit(unit_id))
		var first := _squad(1, [units[0], units[1], units[2]], places.store)
		var second := _squad(2, [units[3], units[4], units[5], units[6]], places.store)
		for unit in units:
			UnitHealth.put_one(first, unit, 10)
		UnitHealth.hurt_one(second, units[6], 10)  # 7 falls first
		UnitHealth.hurt_one(first, units[1], 20)  # 2: past its constitution
		UnitHealth.hurt_one(second, units[3], 10)  # 4, healed again below
		UnitHealth.hurt_one(first, units[2], 10)  # 3, moved to the second squad below
		UnitHealth.hurt_one(second, units[4], 10)  # 5, no longer living below
		UnitHealth.put_one(second, units[3], UnitHealth.of_one(second, units[3]) + 1)
		UnitState.put(second, units[4], SkirmishUnit.State.TAKEN)
		first.units.erase(units[2])
		UnitState.regather(first)
		second.units.append(units[2])
		UnitState.regather(second)
		places.store.columns.put(units[0].slot, UnitColumns.HP, 0)  # no write through UnitHealth
		var events := []

		FormationDeaths.bury([first, second], 1, events)

		var fell := events.filter(func(e): return e["type"] in ["died", "downed"])
		var order := fell.map(func(e): return [e["type"], e["unit"], e["squad"]])
		assert_array(order).override_failure_message(said).is_equal(
			[["died", 2, 1], ["downed", 7, 2], ["downed", 3, 2]]
		)
		assert_int(UnitHealth.of_one(second, units[6])).is_equal(0)  # the excess lost
		assert_int(units[0].state).is_equal(SkirmishUnit.State.MOVING)
		assert_array(Array(places.store.fallen.take())).override_failure_message(said).is_empty()


func test_hp_cooldown_and_regeneration_left_are_the_store_s_alone() -> void:
	var unit := SkirmishUnit.new()
	assert_bool("hp" in unit).is_false()  # removed, not forwarded
	assert_bool("attack_cooldown" in unit).is_false()
	assert_bool("regeneration_left" in unit).is_false()


## A read view holds nothing to commit: hp is written through a held one.
func test_hp_written_through_a_read_view_asserts() -> void:
	var holder := {"store": NativeKernels.unit_store(null)}
	var unit := _unit(1)
	unit.slot = holder.store.enrol(unit.id)
	var view := UnitView.read(holder, [UnitColumns.HP])
	await (
		assert_error(func(): UnitHealth.put(view, unit, 3))
		. is_runtime_error(
			"Assertion failed: hp is written through a held view (UnitView.hold), not a read one"
		)
	)
