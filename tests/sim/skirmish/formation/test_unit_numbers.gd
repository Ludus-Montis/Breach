extends GdUnitTestSuite
## A unit's numbers held by the crate (specs/39-simulation-in-rust.md step 8b5-i): its sheet
## in the unit store by slot, every number one of SkirmishUnit's own; written whole when
## the store admits it and by each setter after; and written no more once the store lets
## it go.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.max_hp = 10 + unit_id
	unit.courage = 70 + unit_id
	unit.critical = 2.25
	unit.parries = true
	unit.carried_by = 1 << 40
	return unit


func _squad(count: int) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for index in range(count):
		members.append(_unit(index + 1))
	return SkirmishSquad.new(1, "player", 1, 0.0, count, members)


## The unit's numbers as the crate holds them, by name.
func _held(unit: SkirmishUnit, field: Object) -> Dictionary:
	var names: PackedStringArray = field.sheet_names()
	var values: PackedFloat64Array = field.sheet(unit.slot)
	var out := {}
	for index in names.size():
		out[names[index]] = values[index] if index < values.size() else null
	return out


func _own(unit: SkirmishUnit, field: Object) -> Dictionary:
	var out := {}
	for name in field.sheet_names():
		out[name] = float(unit.get(name))
	return out


func test_every_number_the_crate_holds_is_one_of_the_units_own() -> void:
	var field: Object = _squad(1).store.field()
	var names: PackedStringArray = field.sheet_names()
	assert_int(names.size()).is_greater(40)
	var properties := {}
	for property in SkirmishUnit.new().get_property_list():
		properties[property["name"]] = property["type"]
	for name in names:
		var said := "SkirmishUnit has no number %s" % name
		(
			assert_bool(properties.get(name, TYPE_NIL) in [TYPE_INT, TYPE_FLOAT, TYPE_BOOL])
			. override_failure_message(said)
			. is_true()
		)


func test_an_admitted_unit_has_its_numbers_in_the_crate() -> void:
	var squad := _squad(3)
	var field: Object = squad.store.field()
	for unit in squad.units:
		assert_dict(_held(unit, field)).is_equal(_own(unit, field))
	assert_float(_held(squad.units[2], field)["courage"]).is_equal(73.0)
	assert_float(_held(squad.units[2], field)["carried_by"]).is_equal(float(1 << 40))


func test_a_number_set_after_is_written_to_the_crate() -> void:
	var squad := _squad(2)
	var field: Object = squad.store.field()
	var unit: SkirmishUnit = squad.units[1]
	unit.wounded = 3
	unit.playing_dead = true
	unit.fresh_speed = 0.1 + 0.2
	var held := _held(unit, field)
	assert_float(held["wounded"]).is_equal(3.0)
	assert_float(held["playing_dead"]).is_equal(1.0)
	assert_float(held["fresh_speed"]).is_equal(0.1 + 0.2)
	assert_dict(held).is_equal(_own(unit, field))


func test_a_unit_let_go_writes_no_more_to_its_old_slot() -> void:
	var squad := _squad(2)
	var store := squad.store
	var field: Object = store.field()
	var gone: SkirmishUnit = squad.units[1]
	var was := gone.slot
	squad.units.erase(gone)
	store.settle([squad])
	var after := _unit(9)
	after.slot = store.admit(after)
	assert_int(after.slot).is_equal(was)
	gone.courage = 1
	assert_float(_held(after, field)["courage"]).is_equal(79.0)
	assert_int(gone.courage).is_equal(1)
