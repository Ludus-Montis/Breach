class_name ColumnsLent
extends RefCounted
## A native call that reads or writes the unit store's columns while a pass holds them
## (ColumnsHeld, Decision 144): the held copies are what the pass sees, so the call is lent
## them - the held values it touches go to the store before it and are read back after it
## (spec 39 step 4b: the entries, the keys, the trade and the locks run in the crate, called
## from passes that hold some of their columns). On columns not held each just runs the call.

const ColumnsHeld = preload("res://sim/skirmish/formation/columns_held.gd")


## The native BodyField `columns` (a store's, held or not) live on.
static func field(columns: UnitColumns) -> Object:
	return columns.inner.field() if columns is ColumnsHeld else columns.field()


## Runs `call`, a native call reading or writing `slot`'s columns `names` (an entry given or
## taken away, a key kept), lent their held values; returns what it returns.
static func slot(columns: UnitColumns, at: int, names: Array, call: Callable) -> Variant:
	if not columns is ColumnsHeld:
		return call.call()
	var copies: Dictionary = columns.copies
	var lending := names.filter(func(name): return copies.has(name) and at < copies[name].size())
	for name in lending:
		columns.inner.put(at, name, copies[name][at])
	var out: Variant = call.call()
	for name in lending:
		copies[name][at] = columns.inner.value(at, name)
	return out


## Runs `call`, a native call reading or writing many slots' columns (a squad's trade, a
## lock), lent every held column; returns what it returns. The copies are read back in
## place, so the arrays the pass handed out stay its own.
static func all(columns: UnitColumns, call: Callable) -> Variant:
	if not columns is ColumnsHeld:
		return call.call()
	columns.commit_held()
	var out: Variant = call.call()
	var copies: Dictionary = columns.copies
	for name in copies:
		# store: per squad (each held column read back once a call, by name, not per unit)
		var fresh: Variant = columns.inner.column(name)
		var copy: Variant = copies[name]
		for index in range(fresh.size()):
			copy[index] = fresh[index]
	return out
