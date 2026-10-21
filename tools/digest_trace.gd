extends RefCounted
## One tick of a digest run as a line of JSON (spec 37): the run, the tick, its events and
## every squad and unit by their field names (DigestState). Values are the digest's plain
## values, vectors as [x, y], floats in their shortest form that reads back bit for bit
## (var_to_str's; JSON.stringify's full precision rounds the last digit wrong).

const DigestState = preload("res://tools/digest_state.gd")


## `events` and `state` as the digest hashes them: DigestState.plain(events) and
## DigestState.state(...). `factions`: each squad's side, in the state's order, for the page
## to colour by; written beside the named fields, not hashed.
static func line(run: String, tick: int, events: Array, state: Array, factions: Array) -> String:
	var squads := []
	for index in range(state.size()):
		var named := {"faction": factions[index]}
		_name(DigestState.SQUAD_FIELDS, state[index], named)
		var units := []
		for unit in named["units"]:
			var named_unit := {}
			_name(DigestState.UNIT_FIELDS, unit, named_unit)
			units.append(named_unit)
		named["units"] = units
		squads.append(named)
	return _json({"run": run, "tick": tick, "events": events, "squads": squads})


## Each value under its name, nested names zipped with nested values, into one dictionary.
static func _name(names: Array, values: Array, out: Dictionary) -> void:
	for index in range(names.size()):
		if names[index] is Array:
			_name(names[index], values[index], out)
		else:
			out[names[index]] = values[index]


## The value as JSON text: vectors as [x, y], floats bit for bit, Infinity and NaN as
## Python's json reads them.
static func _json(value: Variant) -> String:
	if value is Vector2 or value is Vector2i:
		return _json([value.x, value.y])
	if value is Array:
		return "[%s]" % ",".join(value.map(func(v): return _json(v)))
	if value is Dictionary:
		var pairs := PackedStringArray()
		for key in value:
			pairs.append("%s:%s" % [JSON.stringify(str(key)), _json(value[key])])
		return "{%s}" % ",".join(pairs)
	if value is float:
		if is_nan(value):
			return "NaN"
		if is_inf(value):
			return "Infinity" if value > 0 else "-Infinity"
		return var_to_str(value)
	if value is StringName:
		return JSON.stringify(String(value))
	return JSON.stringify(value)
