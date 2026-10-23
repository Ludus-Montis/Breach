class_name SquadRecords
extends RefCounted
## The squads' records in the crate's squad store (spec 39 step 4b): each squad's loose and
## fleeing units, the line it re-faced to, the foes on its other edges and its units out
## chasing. SkirmishSquad's `loose`, `fleeing`, `stance`, `flank_contacts` and `chasers` read
## them here as the dictionaries they were (unit id -> the unit; {"anchor", "heading",
## "foe"}; edge -> {"foe", "since"}; unit id -> {"unit", "foe", "from", "leash"} and
## "returning" once it has turned back), each in the
## order its entries were given. A view is taken once and kept until a call changes any
## record (the field's `records_edits`), so it must never be changed in place: records change
## only through the field's calls (SquadEntries, the locks, StoredSquad's own).

const LOOSE := 0
const FLEEING := 1
const STANCE := 2
const FLANKS := 3
const CHASERS := 4

var _field: Object
var _store  # the UnitStore: a unit's record by its id
var _seen := -1
var _held := {}  # Vector2i(kind, squad slot) -> its view


func _init(field: Object, store) -> void:
	_field = field
	_store = store


## The squad's loose units: unit id -> the unit, in the order each went loose.
func loose(slot: int) -> Dictionary:
	return _view(LOOSE, slot)


## The squad's fleeing units: unit id -> the unit, in the order each began to flee.
func fleeing(slot: int) -> Dictionary:
	return _view(FLEEING, slot)


## The line the squad re-faced to, {"anchor", "heading", "foe"}; {} if none.
func stance(slot: int) -> Dictionary:
	return _view(STANCE, slot)


## The foes on the squad's other edges: edge -> {"foe": squad id, "since": tick}.
func flanks(slot: int) -> Dictionary:
	return _view(FLANKS, slot)


## The squad's units out chasing: unit id -> {"unit", "foe", "from", "leash"}.
func chasers(slot: int) -> Dictionary:
	return _view(CHASERS, slot)


## Sets the squad's stance ({} for none; a stance with no foe faces squad 0, none).
func put_stance(slot: int, stance: Dictionary) -> void:
	if stance.is_empty():
		_field.put_squad_stance(slot, PackedFloat64Array())
		return
	var anchor: Vector2 = stance["anchor"]
	var held := [anchor.x, anchor.y, stance["heading"], stance.get("foe", 0)]
	_field.put_squad_stance(slot, PackedFloat64Array(held))


## Puts the records of the squad in `slot` of `from` (another store's SquadRecords) into
## `to` here, as they stand: a squad carried to another store.
func carry(from: SquadRecords, slot: int, to: int) -> void:
	var loose_ids := PackedInt64Array(from.loose(slot).keys())
	var fleeing_ids := PackedInt64Array(from.fleeing(slot).keys())
	_field.put_squad_entries(to, loose_ids, fleeing_ids)
	put_stance(to, from.stance(slot))
	var flank := from.flanks(slot)
	for edge in flank:
		_field.put_squad_flank(to, edge, flank[edge]["foe"], flank[edge]["since"])
	var chase := from.chasers(slot)
	for unit_id in chase:
		var held: Dictionary = chase[unit_id]
		_field.put_squad_chaser(to, unit_id, held["foe"], held["from"], held["leash"])
		if held.get("returning", false):
			_field.return_squad_chaser(to, unit_id)


func _view(kind: int, slot: int) -> Dictionary:
	var edits: int = _field.records_edits()
	if edits != _seen:
		_seen = edits
		_held.clear()
	var key := Vector2i(kind, slot)
	if not _held.has(key):
		_held[key] = _read(kind, slot)
	return _held[key]


func _read(kind: int, slot: int) -> Dictionary:
	var out := {}
	match kind:
		LOOSE, FLEEING:
			var ids: PackedInt64Array = (
				_field.squad_loose(slot) if kind == LOOSE else _field.squad_fleeing(slot)
			)
			for unit_id in ids:
				out[unit_id] = _store.record_of(unit_id)
		STANCE:
			var held: PackedFloat64Array = _field.squad_stance(slot)
			if not held.is_empty():
				out = {"anchor": Vector2(held[0], held[1]), "heading": held[2], "foe": int(held[3])}
		FLANKS:
			var flat: PackedInt64Array = _field.squad_flanks(slot)
			for index in range(0, flat.size(), 3):
				out[flat[index]] = {"foe": flat[index + 1], "since": flat[index + 2]}
		CHASERS:
			var flat: PackedFloat64Array = _field.squad_chasers(slot)
			for index in range(0, flat.size(), 6):
				var unit_id := int(flat[index])
				out[unit_id] = {
					"unit": _store.record_of(unit_id),
					"foe": int(flat[index + 1]),
					"from": Vector2(flat[index + 2], flat[index + 3]),
					"leash": flat[index + 4],
				}
				if flat[index + 5] != 0.0:
					out[unit_id]["returning"] = true
	return out
