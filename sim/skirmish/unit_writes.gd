class_name UnitWrites
extends RefCounted
## How a unit's numbers, words and lists reach the crate while a store holds it (spec 39
## steps 8b5-i and 8b5-ii, native sheet.rs and words.rs): all at once when the store admits
## it, then each by SkirmishUnit's own setter.

## Each of its numbers' index in the crate's sheet, by name: the crate's own order, asked
## once.
static var _sheet_at := {}


## Writes all of `unit`'s numbers, words and lists to `field`, in its slot.
static func all(unit: Object, field: Object) -> void:
	if _sheet_at.is_empty():
		for name in field.sheet_names():
			_sheet_at[StringName(name)] = _sheet_at.size()
	var values := PackedFloat64Array()
	for name: StringName in _sheet_at:
		values.append(float(unit.get(name)))
	field.put_sheet(unit.slot, values)
	for name in field.word_names():
		word(field, unit.slot, StringName(name), unit.get(name))


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
