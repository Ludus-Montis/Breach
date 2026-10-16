class_name ColumnsHeld
extends UnitColumns
## A pass's hold on the columns it reads and writes unit by unit (Decision 144). On the
## native store each `value` or `put` is a call into the field, about 1 us; a pass over
## every router makes thousands. While a pass holds a store, the store's `columns` is this:
## the held columns are copied out once, when the hold is taken, and the columns not held
## pass straight through. Every reader in the pass goes through `store.columns`, so all of
## them see the same values, and `column` hands out the held copy itself: a typed array of
## it (UnitView) is read and written in place, about 0.04 us an index, and every other
## reader sees those writes. Releasing commits every held column and gives the store its
## own columns back (StoreHolds). A slot opened in the pass grows the copies in place and
## starts blank there. Nothing native may read a held column while it is held.

var inner: UnitColumns  # the store's own columns
var view: RefCounted  # the UnitView the store is held under (UnitView.hold)
var _held := {}  # name -> its copy, for the pass


func _init(own: UnitColumns, names: Array) -> void:
	inner = own
	_columns.clear()  # every column is the store's
	for name in names:
		_held[name] = own.column(name)


func column(name: StringName) -> Variant:
	return _held[name] if _held.has(name) else inner.column(name)


func commit(name: StringName, values: Variant) -> void:
	if not _held.has(name):
		inner.commit(name, values)
		return
	assert(values.size() == _held[name].size(), "a column keeps its length")
	var copy: Variant = _held[name]
	for slot in range(values.size()):  # in place: arrays handed out stay the pass's
		copy[slot] = values[slot]


func value(slot: int, name: StringName) -> Variant:
	return _held[name][slot] if _held.has(name) else inner.value(slot, name)


func put(slot: int, name: StringName, data: Variant) -> void:
	if _held.has(name):
		_held[name][slot] = data
	else:
		inner.put(slot, name, data)


func open(slot: int) -> void:
	inner.open(slot)
	_grow(slot)
	for name in _held:
		_held[name][slot] = BLANKS[name]


func reach(slot: int) -> void:
	inner.reach(slot)
	_grow(slot)


## Commits every held column (StoreHolds.release).
func commit_held() -> void:
	for name in _held:
		inner.commit(name, _held[name])


func _grow(slot: int) -> void:
	for name in _held:
		var copy: Variant = _held[name]
		var size: int = copy.size()
		if size <= slot:
			copy.resize(slot + 1)
			for added in range(size, slot + 1):
				copy[added] = BLANKS[name]
