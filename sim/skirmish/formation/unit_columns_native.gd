class_name UnitColumnsNative
extends UnitColumns
## The unit store's columns on the battle's native BodyField (spec 35 steps 1c, 2b, 3, 4a,
## 5b, 6c and 7b): the field holds every column by slot, so its passes read them where they
## live. `column` is one copy out of the field and `commit` one copy in; `value` and `put` are
## one call each.
## The same contract as UnitColumns (test_unit_store.gd, test_entry_keys.gd).

## Each column's kind on the field: the BodyField calls that reach it.
const INTS := {
	FLAGS: true,
	TOUCH: true,
	KEYS: true,
	RANK: true,
	COLUMN: true,
	HP: true,
	ATTACK_COOLDOWN: true,
	EXERTED: true,
	WAS_RUNNING: true,
	LAST_WOUNDING: true,
}
const LONGS := {GOAL_FOE: true, GOAL_SLOT: true, CAUGHT: true, HELD_BY: true}
const REALS := {
	STUCK: true,
	CLOSEST: true,
	ALONG: true,
	FANNED: true,
	BEARING: true,
	REGENERATION_LEFT: true,
	STAMINA: true,
	SPEED: true,
	BREATHER: true,
	RUN_BUILD: true,
	CONDITION: true,
	REGENERATION_CARRY: true,
	REGENERATION_HALT: true,
	WAKE_LEFT: true,
	DRAIN_CARRY: true,
}

var _field: Object


func _init(field: Object) -> void:
	_field = field
	_columns.clear()  # every column is the field's


func column(name: StringName) -> Variant:
	if INTS.has(name):
		return _field.column_ints(name)
	if LONGS.has(name):
		return _field.column_longs(name)
	if REALS.has(name):
		return _field.column_reals(name)
	return _field.column_points(name)


func commit(name: StringName, values: Variant) -> void:
	if INTS.has(name):
		_field.commit_ints(name, values)
	elif LONGS.has(name):
		_field.commit_longs(name, values)
	elif REALS.has(name):
		_field.commit_reals(name, values)
	else:
		_field.commit_points(name, values)


func value(slot: int, name: StringName) -> Variant:
	if INTS.has(name) or LONGS.has(name):
		return _field.int_of(slot, name)
	if REALS.has(name):
		return _field.real_of(slot, name)
	return _field.point(slot, name)


func put(slot: int, name: StringName, data: Variant) -> void:
	if INTS.has(name) or LONGS.has(name):
		_field.put_int(slot, name, data)
	elif REALS.has(name):
		_field.put_real(slot, name, data)
	else:
		_field.put_point(slot, name, data)


func open(slot: int) -> void:
	_field.blank(slot)
