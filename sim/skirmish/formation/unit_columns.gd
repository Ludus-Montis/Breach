class_name UnitColumns
extends RefCounted
## The unit store's columns (specs/35-unit-store.md): each unit's per-tick state as packed
## arrays indexed by its slot. A pass asks once for each column it reads and indexes it by
## `unit.slot`; it writes one value with put, or a whole column with commit. A column
## fetched before a write doesn't see it: fetch again, or write to the fetched array and
## commit it.

const POSITION := &"position"  # where the unit stands, in cells (spec 27)

## Each column's value for a slot no unit has used yet.
const BLANKS := {POSITION: Vector2.ZERO}

var _columns := {POSITION: PackedVector2Array()}


## The column, by slot.
func column(name: StringName) -> Variant:
	return _columns[name]


## Replaces the column with `values` (by slot, as long as the column).
func commit(name: StringName, values: Variant) -> void:
	assert(values.size() == _columns[name].size(), "a column keeps its length")
	_columns[name] = values


## The value of one unit's slot.
func value(slot: int, name: StringName) -> Variant:
	return _columns[name][slot]


## Sets the value of one unit's slot.
func put(slot: int, name: StringName, data: Variant) -> void:
	_columns[name][slot] = data


## Readies a slot for a unit just given it: every column blank there.
func open(slot: int) -> void:
	for name in _columns:
		if _columns[name].size() <= slot:
			_columns[name].resize(slot + 1)
		_columns[name][slot] = BLANKS[name]
