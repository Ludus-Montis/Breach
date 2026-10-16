extends "res://sim/skirmish/formation/unit_columns_native.gd"
## The native store's columns, counted (spec 35 step 5b): every whole-column copy out of
## the field (`column`) and into it (`commit`), with the slots it moved, and every
## scattered `value` and `put`, filed under the phase `tick_phases` is timing. Counting
## changes no outcome: each call is passed on as it came. `tick_phases copies=true` swaps
## it in for the battle's columns; it is never used by the simulation itself.

var tally := {}  # phase -> calls of each kind, and the slots copied
var names := {}  # phase -> {column name: whole-column copies}
var _pending := {"column": 0, "commit": 0, "slots": 0, "value": 0, "put": 0}
var _pending_names := {}


func column(name: StringName) -> Variant:
	var values: Variant = super.column(name)
	_count("column", values.size())
	_named(name)
	return values


func commit(name: StringName, values: Variant) -> void:
	_count("commit", values.size())
	_named(name)
	super.commit(name, values)


func value(slot: int, name: StringName) -> Variant:
	_count("value", 0)
	return super.value(slot, name)


func put(slot: int, name: StringName, data: Variant) -> void:
	_count("put", 0)
	super.put(slot, name, data)


## Files what was counted since the last lap under `phase`, the one that has just ended.
func file(phase: String) -> void:
	if not tally.has(phase):
		tally[phase] = {"column": 0, "commit": 0, "slots": 0, "value": 0, "put": 0}
		names[phase] = {}
	for kind in _pending:
		tally[phase][kind] += _pending[kind]
		_pending[kind] = 0
	for name in _pending_names:
		names[phase][name] = names[phase].get(name, 0) + _pending_names[name]
	_pending_names.clear()


func _count(kind: String, slots: int) -> void:
	_pending[kind] += 1
	_pending["slots"] += slots


func _named(name: StringName) -> void:
	_pending_names[name] = _pending_names.get(name, 0) + 1


## Each counted phase's copies a tick over `ticks` ticks, the most slots moved first.
func report(ticks: int) -> PackedStringArray:
	var out := PackedStringArray(["copies a tick (column + commit: calls, slots; value, put):"])
	var phases := tally.keys()
	phases.sort_custom(func(a, b): return tally[a]["slots"] > tally[b]["slots"])
	for each in phases:
		var row: Dictionary = tally[each]
		if row["column"] + row["commit"] + row["value"] + row["put"] == 0:
			continue
		var line := "  %-32s %6.1f + %5.1f  %9.0f  %8.0f %8.0f"
		(
			out
			. append(
				(
					line
					% [
						each,
						row["column"] / float(ticks),
						row["commit"] / float(ticks),
						row["slots"] / float(ticks),
						row["value"] / float(ticks),
						row["put"] / float(ticks),
					]
				)
			)
		)
		if not names[each].is_empty():
			var most: Array = names[each].keys()
			most.sort_custom(func(a, b): return names[each][a] > names[each][b])
			var parts := PackedStringArray()
			for name in most.slice(0, 6):
				parts.append("%s %.0f" % [name, names[each][name] / float(ticks)])
			out.append("      " + ", ".join(parts))
	return out
