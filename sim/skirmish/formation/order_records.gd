class_name OrderRecords
extends RefCounted
## The squads' pursuits and withdrawals in the crate's squad store (spec 39 step 7c), set by
## the crate's orders, pursuit and withdrawal. SkirmishSquad's `pursuit` and `withdraw` read
## them here as the dictionaries they were ({"foe", "route", "post", "post_at", "home",
## "direction", "order", "returning"}; {"safe_ticks"}). The route a pursuer held its post on
## is kept here (`hold_route`) until routes are the crate's. A view is taken once and kept
## until a call changes any record (the field's `records_edits`), as SquadRecords' are, so it
## must never be changed in place.

const PURSUIT := 0
const WITHDRAW := 1

var _field: Object
var _seen := -1
var _held := {}  # Vector2i(kind, squad slot) -> its view
var _routes := {}  # squad slot -> the route its pursuit left its post on


func _init(field: Object) -> void:
	_field = field


## The squad's pursuit as a whole; {} if none.
func pursuit(slot: int) -> Dictionary:
	return _view(PURSUIT, slot)


## The squad's withdrawal, {"safe_ticks"}; {} if none.
func withdraw(slot: int) -> Dictionary:
	return _view(WITHDRAW, slot)


## Keeps the route the squad's pursuit left its post on.
func hold_route(slot: int, route) -> void:
	_routes[slot] = route
	_held.erase(Vector2i(PURSUIT, slot))


## Puts the pursuit and withdrawal of the squad in `slot` of `from` (another store's
## OrderRecords) into `to` here, as they stand: a squad carried to another store.
func carry(from: OrderRecords, slot: int, to: int) -> void:
	_field.put_squad_pursuit(to, from._field.squad_pursuit(slot))
	hold_route(to, from._routes.get(slot))
	_field.put_squad_withdraw(to, from._field.squad_withdraw(slot))


func _view(kind: int, slot: int) -> Dictionary:
	var edits: int = _field.records_edits()
	if edits != _seen:
		_seen = edits
		_held.clear()
	var key := Vector2i(kind, slot)
	if not _held.has(key):
		_held[key] = _pursuit(slot) if kind == PURSUIT else _withdraw(slot)
	return _held[key]


func _pursuit(slot: int) -> Dictionary:
	var held: PackedFloat64Array = _field.squad_pursuit(slot)
	if held.is_empty():
		return {}
	return {
		"foe": int(held[0]),
		"route": _routes.get(slot),
		"post": held[1],
		"post_at": Vector2(held[2], held[3]),
		"home": held[4],
		"direction": int(held[5]),
		"order": int(held[6]),
		"returning": held[7] != 0.0,
	}


func _withdraw(slot: int) -> Dictionary:
	var ticks: int = _field.squad_withdraw(slot)
	return {} if ticks < 0 else {"safe_ticks": ticks}
