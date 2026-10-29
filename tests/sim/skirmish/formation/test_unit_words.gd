extends GdUnitTestSuite
## A unit's words and lists held by the crate (specs/39-simulation-in-rust.md step 8b5-ii):
## each one of SkirmishUnit's own, in the unit store by slot, coded for the crate; written
## whole when the store admits it and by each setter after (a trait set in place too); and
## written no more once the store lets it go.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


func _unit(unit_id: int) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = "player"
	unit.damage_type = "pierce"
	unit.tags.assign(["infantry", "veteran %d" % unit_id])
	unit.attributes = {"strength": 10 + unit_id, "wits": 8}
	unit.traits = {"darksight": 2, "messenger": unit_id}
	unit.melee_parts = [[4.0, "slash", false], [1.5, "fire", true]]
	unit.immunities.assign(["fire"])
	return unit


func _squad(count: int) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for index in range(count):
		members.append(_unit(index + 1))
	return SkirmishSquad.new(1, "player", 1, 0.0, count, members)


## The unit's words and lists as it holds them, numbers as floats (as the crate reads them).
func _own(unit: SkirmishUnit, field: Object) -> Dictionary:
	var out := {}
	for name in field.word_names():
		var value: Variant = unit.get(name)
		if value is Dictionary:
			var levels := {}
			for word in value:
				levels[word] = float(value[word])
			value = levels
		elif name.ends_with("_parts"):
			value = value.map(func(part): return [float(part[0]), part[1], bool(part[2])])
		elif value is Array:
			value = PackedStringArray(value)
		out[name] = value
	return out


func test_every_word_the_crate_holds_is_one_of_the_units_own() -> void:
	var field: Object = _squad(1).store.field()
	var names: PackedStringArray = field.word_names()
	assert_int(names.size()).is_equal(12)
	var properties := {}
	for property in SkirmishUnit.new().get_property_list():
		properties[property["name"]] = property["type"]
	for name in names:
		var said := "SkirmishUnit has no word or list %s" % name
		(
			assert_bool(
				properties.get(name, TYPE_NIL) in [TYPE_STRING, TYPE_ARRAY, TYPE_DICTIONARY]
			)
			. override_failure_message(said)
			. is_true()
		)


func test_an_admitted_unit_has_its_words_in_the_crate() -> void:
	var squad := _squad(3)
	var field: Object = squad.store.field()
	for unit in squad.units:
		assert_dict(field.words(unit.slot)).is_equal(_own(unit, field))
	var held: Dictionary = field.words(squad.units[2].slot)
	assert_str(held["faction_id"]).is_equal("player")
	assert_array(Array(held["tags"])).is_equal(["infantry", "veteran 3"])
	assert_float(held["attributes"]["strength"]).is_equal(13.0)
	assert_array(held["melee_parts"][1]).is_equal([1.5, "fire", true])


func test_a_word_or_list_set_after_is_written_to_the_crate() -> void:
	var squad := _squad(2)
	var field: Object = squad.store.field()
	var unit: SkirmishUnit = squad.units[1]
	unit.damage_type = "crush"
	var stops: Array[String] = ["acid", "fire"]
	unit.regeneration_stops = stops
	unit.ranged_parts = [[2.0, "pierce", false]]
	unit.set_trait("messenger", 0)
	var held: Dictionary = field.words(unit.slot)
	assert_str(held["damage_type"]).is_equal("crush")
	assert_array(Array(held["regeneration_stops"])).is_equal(["acid", "fire"])
	assert_float(held["traits"]["messenger"]).is_equal(0.0)
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
	gone.faction_id = "kingdom"
	gone.set_trait("darksight", 5)
	assert_dict(field.words(was)).is_equal(_own(after, field))
	assert_str(gone.faction_id).is_equal("kingdom")
