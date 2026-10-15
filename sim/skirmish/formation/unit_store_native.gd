class_name UnitStoreNative
extends UnitStore
## The unit store on the battle's native BodyField (Decision 129): its bodies are the
## store's slots. A unit the store holds keeps its slot off the field (dead, downed), and
## the field enrols it again when a sync names it.

var _field: Object


func _init(field: Object) -> void:
	_field = field


func enrol(unit_id: int) -> int:
	var held: int = _field.slot_of(unit_id)
	if held >= 0:
		return held
	var slot: int = _field.hold(unit_id)
	columns.open(slot)
	return slot


func drop(unit_id: int) -> void:
	_field.release(unit_id)


func slot_of(unit_id: int) -> int:
	return _field.slot_of(unit_id)


func id_at(slot: int) -> int:
	return _field.unit_at(slot)


func count() -> int:
	return _field.held_count()


func ids() -> PackedInt64Array:
	return _field.held_units()
