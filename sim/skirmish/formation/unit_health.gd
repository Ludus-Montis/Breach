class_name UnitHealth
extends RefCounted
## A unit's hp in the unit store's HP column (spec 35 step 5b, Decision 145), and its one
## writer: every change to a unit's hp goes through `hurt`, `heal` or `put`
## (check_one_health.py holds that). A write that takes a unit from above 0 to 0 or below
## puts it on the store's fell queue, which FormationDeaths.bury drains; whether it died or
## is downed is decided there, from its hp at the end of the tick's blows.
##
## `source` is anything holding the store (a squad, the simulation) for scattered use, or a
## Table: a pass over many units fetches the column once (`table`), reads and writes the
## copy, and commits it once (`commit`). Under Rust each scattered read or write is a call
## into the field.

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## The HP column fetched once, by slot, with the store it came from: a copy as of now.
class Table:
	var store: UnitStore
	var hp: PackedInt32Array

	func _init(from: UnitStore) -> void:
		store = from
		hp = from.columns.column(UnitColumns.HP)


## The HP column of `source`'s store, fetched once.
static func table(source) -> Table:
	return Table.new(source.store)


## Writes the table's copy back to its store, once.
static func commit(health: Table) -> void:
	health.store.columns.commit(UnitColumns.HP, health.hp)


## The unit's hp.
static func of(source, unit) -> int:
	if source is Table:
		return source.hp[unit.slot]
	return source.store.columns.value(unit.slot, UnitColumns.HP)


## Takes `amount` from the unit's hp.
static func hurt(source, unit, amount: int) -> void:
	put(source, unit, of(source, unit) - amount)


## Adds `amount` to the unit's hp.
static func heal(source, unit, amount: int) -> void:
	put(source, unit, of(source, unit) + amount)


## Sets the unit's hp to `value`; a unit taken from above 0 to 0 or below falls.
static func put(source, unit, value: int) -> void:
	var was := of(source, unit)
	if source is Table:
		source.hp[unit.slot] = value
	else:
		source.store.columns.put(unit.slot, UnitColumns.HP, value)
	if was > 0 and value <= 0:
		source.store.fallen.push(unit.id)
