extends GdUnitTestSuite
## A unit made in the crate from its definition (specs/39-simulation-in-rust.md step 8c-iii,
## native made.rs and arms.rs): for every unit in content/units/, its numbers, words and lists
## and walker are those FormationUnits.make gives it.

const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UNITS := "res://content/units/"


## Each unit's id in content/units/.
func _ids() -> PackedStringArray:
	var ids := PackedStringArray()
	for file in DirAccess.get_files_at(UNITS):
		if file.ends_with(".json"):
			ids.append(file.get_basename())
	return ids


## `unit`'s words and lists as the crate decodes them: numbers as floats, lists packed.
func _said(unit: SkirmishUnit, names: PackedStringArray) -> Dictionary:
	var out := {}
	for name in names:
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


func test_every_unit_the_crate_makes_is_the_one_formation_units_makes() -> void:
	var field: Object = NativeKernels.body_field()
	var ids := _ids()
	assert_int(ids.size()).is_greater(5)
	for id in ids:
		var unit := FormationUnits.make(CrateContent.unit(id), "kingdom", -1, 0)
		var made: Dictionary = CrateContent.store().made(id, "kingdom", -1)
		var sheet := PackedFloat64Array()
		for name in field.sheet_names():
			sheet.append(float(unit.get(name)))
		var said := "%s: " % id
		assert_array(made.get("sheet", [])).override_failure_message(said + "sheet").is_equal(sheet)
		var words := _said(unit, field.word_names())
		assert_dict(made.get("words", {})).override_failure_message(said + "words").is_equal(words)
		var walker: PackedFloat64Array = unit.walker.fields()
		assert_array(made.get("walker", [])).override_failure_message(said + "walker").is_equal(
			walker
		)


func test_no_unit_is_made_for_a_definition_that_isnt_there() -> void:
	assert_dict(CrateContent.store().made("no_such_unit", "kingdom", 1)).is_empty()
