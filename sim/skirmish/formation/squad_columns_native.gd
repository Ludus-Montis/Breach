class_name SquadColumnsNative
extends SquadColumns
## The squad store on the battle's native BodyField (spec 39 step 1): the crate holds every
## squad's columns and members (`Squads`), and writes them with the rest of its state
## (`snapshot`). Each read or write is one call. The same contract as SquadColumns
## (test_squad_store.gd).

var _field: Object


func _init(field: Object) -> void:
	_field = field


func open(squad_id: int) -> int:
	if opened >= 0:
		return _take_opened()
	return _field.squad_open(squad_id)


func count() -> int:
	return _field.squad_count()


func value(slot: int, column: int) -> Variant:
	if column < REALS:
		return _field.squad_int(slot, column)
	if column < POINTS:
		return _field.squad_real(slot, column)
	return _field.squad_point(slot, column)


func put(slot: int, column: int, data: Variant) -> void:
	if column < REALS:
		_field.put_squad_int(slot, column, data)
	elif column < POINTS:
		_field.put_squad_real(slot, column, data)
	else:
		_field.put_squad_point(slot, column, data)


func members(slot: int) -> PackedInt32Array:
	return _field.squad_members(slot)


func put_members(slot: int, slots: PackedInt32Array) -> void:
	_field.put_squad_members(slot, slots)
