class_name UnitColumnsNative
extends UnitColumns
## The unit store's columns on the battle's native BodyField (spec 35 step 1c): the field
## holds them by slot, so its syncs read them where they live. `column` is one copy out of
## the field and `commit` one copy in; `value` and `put` are one call each. The same
## contract as UnitColumns (test_unit_store.gd).

var _field: Object


func _init(field: Object) -> void:
	_field = field


func column(name: StringName) -> Variant:
	return _field.column_flags() if name == FLAGS else _field.column_points(name)


func commit(name: StringName, values: Variant) -> void:
	if name == FLAGS:
		_field.commit_flags(values)
	else:
		_field.commit_points(name, values)


func value(slot: int, name: StringName) -> Variant:
	return _field.flags_of(slot) if name == FLAGS else _field.point(slot, name)


func put(slot: int, name: StringName, data: Variant) -> void:
	if name == FLAGS:
		_field.put_flags(slot, data)
	else:
		_field.put_point(slot, name, data)


func open(slot: int) -> void:
	_field.blank(slot)
