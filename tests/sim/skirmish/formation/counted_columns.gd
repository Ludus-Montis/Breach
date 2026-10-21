## A store's columns with every whole-column read counted by name (Decision 150).
extends "res://sim/skirmish/formation/unit_columns.gd"
var inner
var reads := {}


func _init(own) -> void:
	inner = own
	_columns.clear()  # every column is the store's


func column(name: StringName) -> Variant:
	reads[name] = reads.get(name, 0) + 1
	return inner.column(name)


func commit(name: StringName, values: Variant) -> void:
	inner.commit(name, values)


func value(slot: int, name: StringName) -> Variant:
	return inner.value(slot, name)


func put(slot: int, name: StringName, data: Variant) -> void:
	inner.put(slot, name, data)


func open(slot: int) -> void:
	inner.open(slot)


func reach(slot: int) -> void:
	inner.reach(slot)
