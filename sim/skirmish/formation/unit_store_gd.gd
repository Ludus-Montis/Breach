class_name UnitStoreGd
extends UnitStore
## The unit store in GDScript, the reference (Decision 129): a map from unit ids to slots,
## the ids by slot, and the free slots in ascending order.

const FREE := -1

var _slots := {}  # unit id -> slot
var _ids := PackedInt64Array()  # slot -> unit id, FREE if none
var _free := PackedInt32Array()  # ascending


func enrol(unit_id: int) -> int:
	if _slots.has(unit_id):
		return _slots[unit_id]
	var slot := _ids.size()
	if _free.is_empty():
		_ids.append(unit_id)
	else:
		slot = _free[0]
		_free.remove_at(0)
		_ids[slot] = unit_id
	_slots[unit_id] = slot
	return slot


func drop(unit_id: int) -> void:
	if not _slots.has(unit_id):
		return
	var slot: int = _slots[unit_id]
	_slots.erase(unit_id)
	_ids[slot] = FREE
	_free.insert(_free.bsearch(slot), slot)


func slot_of(unit_id: int) -> int:
	return _slots.get(unit_id, FREE)


func id_at(slot: int) -> int:
	return _ids[slot] if slot >= 0 and slot < _ids.size() else FREE


func count() -> int:
	return _slots.size()


func ids() -> PackedInt64Array:
	var out := PackedInt64Array()
	for unit_id in _ids:
		if unit_id != FREE:
			out.append(unit_id)
	return out
