class_name UnitWrites
extends RefCounted
## How a unit's numbers, words, lists and objects reach the crate while a store holds it
## (spec 39 steps 8b5-i to 8b5-iii, native sheet.rs, words.rs and objects.rs): all at once
## when the store admits it, then each by SkirmishUnit's own setter.

## Each of its numbers' index in the crate's sheet, by name: the crate's own order, asked
## once.
static var _sheet_at := {}


## Writes all of `unit`'s numbers, words, lists and objects to `store`'s field, in its slot.
static func all(unit: Object, store: Object) -> void:
	var field: Object = store.field()
	if _sheet_at.is_empty():
		for name in field.sheet_names():
			_sheet_at[StringName(name)] = _sheet_at.size()
	var values := PackedFloat64Array()
	for name: StringName in _sheet_at:
		values.append(float(unit.get(name)))
	field.put_sheet(unit.slot, values)
	for name in field.word_names():
		word(field, unit.slot, StringName(name), unit.get(name))
	for name in field.object_names():
		object(store, unit.slot, StringName(name), unit.get(name))


## Writes the number `name` in `slot`.
static func number(field: Object, slot: int, name: StringName, value: Variant) -> void:
	field.put_sheet_value(slot, _sheet_at[name], float(value))


## Writes the word or list `name` in `slot`, by its shape: a word, words with levels,
## weapons' parts or a list of words.
static func word(field: Object, slot: int, name: StringName, value: Variant) -> void:
	if value is String:
		field.put_word(slot, name, value)
	elif value is Dictionary:
		field.put_word_levels(slot, name, value)
	elif name == &"melee_parts" or name == &"ranged_parts":
		field.put_word_parts(slot, name, value)
	else:
		field.put_word_list(slot, name, PackedStringArray(value))


## Writes the object `name` in `slot`, coded: a walker's fields (TerrainWalker.fields), a
## command's slot (FormationCommand.slot) or a definition's code (UnitStore.definition_code),
## with the definition's sight, which a look reads (spec 39 step 8b5-vii), and its speed, which
## a bearer's pace is a share of (8b5-xxix); none is empty or -1.
static func object(store: Object, slot: int, name: StringName, value: Variant) -> void:
	var field: Object = store.field()
	if name == &"walker":
		field.put_walker(slot, PackedFloat64Array() if value == null else value.fields())
	elif name == &"definition":
		var code: int = store.definition_code(value)
		field.put_object(slot, name, code)
		if value != null:
			var sight := [value.sight_ahead, value.sight_side, value.sight_behind]
			field.put_sight(code, PackedFloat64Array([value.detection_range] + sight))
			field.put_speed(code, value.speed)
	else:
		field.put_object(slot, name, -1 if value == null else value.slot)
