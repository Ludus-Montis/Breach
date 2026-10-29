extends GdUnitTestSuite
## A unit's objects held by the crate (specs/39-simulation-in-rust.md step 8b5-iii): its
## walker, the command it follows and the one it started under, and its definition, each one
## of SkirmishUnit's own, coded for the crate in the unit store by slot; written whole when
## the store admits it and by each setter after; and written no more once the store lets it
## go.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _unit(unit_id: int, definition: UnitDef = null) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.walker = TerrainWalker.new(1.0 + unit_id, unit_id, 2, true, unit_id % 2 == 0)
	unit.definition = definition
	return unit


func _squad(definitions: Array) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for index in range(definitions.size()):
		members.append(_unit(index + 1, definitions[index]))
	return SkirmishSquad.new(1, "player", 1, 0.0, members.size(), members)


## The unit's objects as the crate codes them.
func _own(unit: SkirmishUnit, store: UnitStore) -> Dictionary:
	return {
		"walker": PackedFloat64Array() if unit.walker == null else unit.walker.fields(),
		"command": -1 if unit.command == null else unit.command.slot,
		"origin": -1 if unit.origin == null else unit.origin.slot,
		"definition": store.definition_code(unit.definition),
	}


func test_every_object_the_crate_holds_is_one_of_the_units_own() -> void:
	var field: Object = _squad([null]).store.field()
	var names: PackedStringArray = field.object_names()
	assert_array(Array(names)).is_equal(["walker", "command", "origin", "definition"])
	var properties := {}
	for property in SkirmishUnit.new().get_property_list():
		properties[property["name"]] = property["type"]
	for name in names:
		var said := "SkirmishUnit has no object %s" % name
		assert_int(properties.get(name, TYPE_NIL)).override_failure_message(said).is_equal(
			TYPE_OBJECT
		)


func test_an_admitted_unit_has_its_objects_in_the_crate() -> void:
	var archer := UnitDef.new()
	var squad := _squad([archer, UnitDef.new(), archer])
	var store := squad.store
	var field: Object = store.field()
	for unit in squad.units:
		assert_dict(field.objects(unit.slot)).is_equal(_own(unit, store))
	var first: Dictionary = field.objects(squad.units[0].slot)
	assert_array(Array(first["walker"])).is_equal([2.0, 1.0, 2.0, 1.0, 0.0])
	assert_int(first["command"]).is_equal(squad.command.slot)
	assert_int(first["origin"]).is_equal(squad.command.slot)
	var codes := squad.units.map(func(unit): return field.objects(unit.slot)["definition"])
	assert_array(codes).is_equal([0, 1, 0])  # the order first seen; one definition, one code
	assert_object(store.definition_of(1)).is_same(squad.units[1].definition)


func test_an_object_set_after_is_written_to_the_crate() -> void:
	var squad := _squad([null, null])
	var store := squad.store
	var field: Object = store.field()
	var unit: SkirmishUnit = squad.units[1]
	assert_int(field.objects(unit.slot)["definition"]).is_equal(-1)
	unit.walker = TerrainWalker.grounded(3.0)
	unit.command = FormationCommand.new(field, 5.0)
	unit.definition = UnitDef.new()
	var held: Dictionary = field.objects(unit.slot)
	assert_array(Array(held["walker"])).is_equal([3.0, 0.0, 0.0, 0.0, 0.0])
	assert_int(held["command"]).is_equal(unit.command.slot).is_not_equal(unit.origin.slot)
	assert_int(held["definition"]).is_equal(0)
	unit.walker = null
	unit.origin = null
	assert_dict(field.objects(unit.slot)).is_equal(_own(unit, store))


func test_a_unit_let_go_writes_no_more_to_its_old_slot() -> void:
	var squad := _squad([null, null])
	var store := squad.store
	var field: Object = store.field()
	var gone: SkirmishUnit = squad.units[1]
	var was := gone.slot
	squad.units.erase(gone)
	store.settle([squad])
	var after := _unit(9)
	after.slot = store.admit(after)
	assert_int(after.slot).is_equal(was)
	gone.walker = TerrainWalker.grounded(7.0)
	gone.definition = UnitDef.new()
	gone.command = squad.command
	assert_dict(field.objects(was)).is_equal(_own(after, store))
	assert_float(gone.walker.height).is_equal(7.0)
