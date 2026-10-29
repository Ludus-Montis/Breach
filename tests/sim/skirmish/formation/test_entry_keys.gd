extends GdUnitTestSuite
## The keys of a unit's loose and fleeing entries in the unit store's columns
## (specs/35-unit-store.md step 2, EntryKeys): each column round-trips and starts blank; an
## entry holds the unit alone, and the store holds exactly the keys it was given, as the
## digest rebuilds them; and a unit moved to another squad leaves no entry behind. The
## native store holds every column on its field (step 2b).

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")


## Each engine that runs here: [name, a new store on it].
func _backends() -> Array:
	var out := [["rust", NativeKernels.unit_store(null)]]
	return out


func test_the_entries_other_keys_round_trip_and_start_blank() -> void:
	var keys := {
		UnitColumns.NEXT: Vector2(1, 2),
		UnitColumns.GOAL_FOE: 7,
		UnitColumns.GOAL_SLOT: 3,
		UnitColumns.FOE_AT: Vector2(4, 4),
		UnitColumns.TOUCH: 1,
		UnitColumns.TOWARD: Vector2(-1, 0),
		UnitColumns.STUCK: 0.3,
		UnitColumns.CLOSEST: 1.25,
		UnitColumns.ALONG: 40.5,
		UnitColumns.CAUGHT: 2,
		UnitColumns.HELD_BY: 9,
		UnitColumns.FANNED: INF,
		UnitColumns.KEYS: 5,
	}
	for backend in _backends():
		var store: UnitStore = backend[1]
		var columns := store.columns
		var slot := store.enrol(5)
		for name in keys:
			var said := "%s %s" % [backend[0], name]
			var blank = UnitColumns.BLANKS[name]
			assert_that(columns.value(slot, name)).override_failure_message(said).is_equal(blank)
			columns.put(slot, name, keys[name])
			assert_that(columns.value(slot, name)).override_failure_message(said).is_equal(
				keys[name]
			)
		store.drop(5)
		assert_int(store.enrol(9)).override_failure_message(backend[0]).is_equal(slot)
		for name in keys:
			assert_that(columns.value(slot, name)).is_equal(UnitColumns.BLANKS[name])


func test_an_entry_holds_exactly_the_keys_it_was_given() -> void:
	var unit := SkirmishUnit.new()
	unit.id = 4
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 1, [unit])
	var columns := squad.store.columns
	SquadEntries.loosen(squad, unit, Vector2(2, 3))
	assert_object(squad.loose[unit.id]).is_same(unit)  # the entry holds the unit alone
	var given := {"unit": unit, "goal": null, "next": Vector2(2, 3), "at": Vector2(2, 3)}
	assert_dict(EntryKeys.loose_entry(columns, unit, Vector2(2, 3))).is_equal(given)
	columns.put(unit.slot, UnitColumns.GOAL_FOE, 7)
	columns.put(unit.slot, UnitColumns.GOAL_SLOT, 3)
	EntryKeys.keep(columns, unit.slot, UnitColumns.TOUCH, 1)
	EntryKeys.keep(columns, unit.slot, UnitColumns.TOWARD, null)
	given.merge({"goal": [7, 3], "touch": true, "toward": null}, true)
	assert_dict(EntryKeys.loose_entry(columns, unit, Vector2(2, 3))).is_equal(given)
	EntryKeys.keep(columns, unit.slot, UnitColumns.TOWARD, Vector2(1, 1))
	assert_that(EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)).is_equal(Vector2(1, 1))
	SquadEntries.loosen(squad, unit, Vector2(5, 5))  # a new entry: none of the old keys
	var fresh := {"unit": unit, "goal": null, "next": Vector2(5, 5), "at": Vector2(5, 5)}
	assert_dict(EntryKeys.loose_entry(columns, unit, Vector2(5, 5))).is_equal(fresh)
	assert_that(EntryKeys.held(columns, unit.slot, UnitColumns.TOWARD)).is_null()
	SquadEntries.flee(squad, unit, 12.5, Vector2(0, 1))
	EntryKeys.keep(columns, unit.slot, UnitColumns.HELD_BY, null)
	var fled := {"along": 12.5, "offset": Vector2(0, 1), "held_by": null}
	assert_dict(EntryKeys.fleeing_entry(columns, unit.slot, Vector2(0, 1))).is_equal(fled)
	assert_object(squad.fleeing[unit.id]).is_same(unit)


func test_a_unit_moved_to_another_squad_leaves_no_entry_behind() -> void:
	var units: Array[SkirmishUnit] = []
	for unit_id in range(1, 5):
		var unit := SkirmishUnit.new()
		unit.id = unit_id
		units.append(unit)
	var own := SkirmishSquad.new(1, "player", 1, 0.0, 2, [units[0], units[1]])
	var best := SkirmishSquad.new(2, "player", 1, 0.0, 2, [units[2], units[3]], null, own.store)
	SquadEntries.loosen(own, units[0], Vector2(1, 1))
	SquadEntries.flee(own, units[0], 3.0, Vector2.ZERO)
	UnitState.put(own, units[0], SkirmishUnit.State.DOWNED)
	own.store.columns.put(units[0].slot, UnitColumns.WAKE_LEFT, 0.0)  # it comes to now
	own.state = SkirmishSquad.State.ROUTING  # so it joins `best`, not its own
	FormationRecovery.step([own, best], 0.1, 1, 3, [])
	assert_bool(own.loose.has(1) or own.fleeing.has(1)).is_false()
	assert_object(best.loose[1]).is_same(units[0])
	assert_int(own.store.columns.value(units[0].slot, UnitColumns.FLAGS)).is_equal(
		UnitColumns.LOOSE
	)
	SquadEntries.loosen(own, units[1], Vector2(2, 2))
	FormationContact.reinforce(best, own)
	assert_bool(own.loose.is_empty()).is_true()
	assert_int(own.store.columns.value(units[1].slot, UnitColumns.FLAGS)).is_equal(0)


func test_the_native_store_holds_every_column_on_its_field() -> void:
	var field := NativeKernels.body_field()
	if field == null:
		print("test_entry_keys: the Rust core isn't built - its half skipped")
		return
	var store := NativeKernels.unit_store(field)
	var slot := store.enrol(3)
	assert_int(field.column_longs(UnitColumns.GOAL_SLOT)[slot]).is_equal(-1)  # blank: no goal
	assert_float(field.column_reals(UnitColumns.CLOSEST)[slot]).is_equal(INF)
	store.columns.put(slot, UnitColumns.LOOSE_AT, Vector2(4, 5))
	store.columns.put(slot, UnitColumns.FLAGS, UnitColumns.LOOSE)
	store.columns.put(slot, UnitColumns.NEXT, Vector2(6, 7))  # the entries' keys (step 2b)
	store.columns.put(slot, UnitColumns.KEYS, 3)
	store.columns.put(slot, UnitColumns.GOAL_SLOT, 2)
	store.columns.put(slot, UnitColumns.ALONG, 12.5)
	assert_vector(field.column_points(UnitColumns.LOOSE_AT)[slot]).is_equal(Vector2(4, 5))
	assert_int(field.column_ints(UnitColumns.FLAGS)[slot]).is_equal(UnitColumns.LOOSE)
	assert_vector(field.column_points(UnitColumns.NEXT)[slot]).is_equal(Vector2(6, 7))
	assert_int(field.column_ints(UnitColumns.KEYS)[slot]).is_equal(3)
	assert_int(field.column_longs(UnitColumns.GOAL_SLOT)[slot]).is_equal(2)
	assert_float(field.column_reals(UnitColumns.ALONG)[slot]).is_equal(12.5)
	assert_object(store.field()).is_same(field)
