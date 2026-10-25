class_name UnitStoreNative
extends UnitStore
## The unit store on the battle's native BodyField (Decision 129): its bodies are the
## store's slots. A unit the store holds keeps its slot off the field (dead, downed), and
## the field enrols it again when a sync names it. Its columns live on the field too
## (UnitColumnsNative), where the field's syncs read them, and so does its fell queue
## (FellQueueNative), and its squad store (SquadColumnsNative) and the squads' records
## (SquadRecords, OrderRecords).

const UnitColumnsNative = preload("res://sim/skirmish/formation/unit_columns_native.gd")
const SquadColumnsNative = preload("res://sim/skirmish/formation/squad_columns_native.gd")

var _field: Object


func _init(native_field: Object) -> void:
	_field = native_field
	columns = UnitColumnsNative.new(native_field)
	squads = SquadColumnsNative.new(native_field)
	records = SquadRecords.new(native_field, self)
	orders = OrderRecords.new(native_field)
	fallen = FellQueueNative.new(native_field)


func field() -> Object:
	return _field


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
