class_name ContentFill
extends RefCounted
## Fills a GDScript definition from the fields the Rust crate loaded (Decision 153, spec 39
## step 2), by name, for CrateContent. Each kind's fields are its properties' names.


## Sets each of `fields` on `made` by name and returns it. An item's shared fields ("item")
## are its own; a typed list keeps its type ("inf" in a list of reals is an infinity, and a
## list of definitions is filled one by one); a name where an enum is kept is the enum's
## value; a colour is its "#rrggbb".
static func fill(made: Object, fields: Dictionary) -> Object:
	for key: String in fields:
		var value: Variant = fields[key]
		var current: Variant = made.get(key)
		if key == "item" and value is Dictionary:
			fill(made, value)
		elif current is Array:
			_fill_list(current, value)
		elif current is Color:
			made.set(key, Color(value))
		elif current is int and value is String:
			made.set(key, _enum_value(made, key, value))
		else:
			made.set(key, value)
	return made


static func _fill_list(list: Array, values: Array) -> void:
	match list.get_typed_builtin():
		TYPE_FLOAT:
			list.assign(values.map(_real))
		TYPE_OBJECT:
			var script: Script = list.get_typed_script()
			list.assign(
				values.map(func(entry: Dictionary) -> Object: return fill(script.new(), entry))
			)
		_:
			list.assign(values)


static func _real(value: Variant) -> float:
	if value is String:
		return -INF if value == "-inf" else INF
	return value


## The value of `made`'s enum `key` named `named` (case aside), from its export's hint.
static func _enum_value(made: Object, key: String, named: String) -> int:
	for property in made.get_property_list():
		if property["name"] == key:
			var names: PackedStringArray = property["hint_string"].split(",")
			for index in names.size():
				var entry := names[index]
				if entry.get_slice(":", 0).to_lower() == named.to_lower():
					return int(entry.get_slice(":", 1)) if ":" in entry else index
	push_error("ContentFill: %s isn't one of %s's values" % [named, key])
	return 0
