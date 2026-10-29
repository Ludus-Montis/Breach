@abstract class_name UnitStore
extends RefCounted
## The battle's unit store (specs/35-unit-store.md, Decision 142): every unit in a list the
## simulation or the digest reads has a dense slot, given when it enrols and freed when it
## is in no list. Free slots are reused lowest first. A slot is storage only: no loop runs
## in slot order and no tie is broken by slot (Decision 97). Its backend is UnitStoreNative,
## on the battle's BodyField under either engine (spec 39 step 4a; test_unit_store.gd),
## made by NativeKernels.unit_store. Columns
## of per-tick state are in `columns` (UnitColumns), indexed by slot; a unit's slot is on
## its record (SkirmishUnit.slot) and reached through its squad (SkirmishSquad.store).
## Its squads' state is the squad store, `squads` (SquadColumns, spec 39 step 1), and their
## records there, `records` (SquadRecords, step 4b), and their pursuits and withdrawals,
## `orders` (OrderRecords, step 7c).
## On the native backend the columns live on the field (UnitColumnsNative), and a native
## pass runs on the field of the squads' own store (`field`).

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadColumns = preload("res://sim/skirmish/formation/squad_columns.gd")

var columns: UnitColumns = UnitColumns.new()
var squads: SquadColumns = SquadColumns.new()  # the squad store (spec 39 step 1)
var records: SquadRecords  # the squads' records there (spec 39 step 4b)
var orders: OrderRecords  # their pursuits and withdrawals (spec 39 step 7c)
var fallen: FellQueue = FellQueue.new()  # the units an hp write took to 0 or below (UnitHealth)
var _records := {}  # unit id -> its record, kept in GDScript


## Enrols a unit's record: its slot, as `enrol`, given it, and the record kept so a unit
## named by id (the fell queue) is found again (`record_of`). Its numbers are held on the
## store's field from now on (SkirmishUnit.hold_sheet, spec 39 step 8b5-i).
func admit(unit) -> int:
	_records[unit.id] = unit
	unit.slot = enrol(unit.id)
	unit.hold_sheet(field())
	return unit.slot


## The record admitted for a unit id, or null.
func record_of(unit_id: int):
	return _records.get(unit_id)


## The native BodyField the store and its columns live on.
func field() -> Object:
	return null


## The unit's slot: its own if it has one, else the lowest free one.
@abstract func enrol(unit_id: int) -> int

## Frees the unit's slot (nothing if it has none).
@abstract func drop(unit_id: int) -> void

## The unit's slot, -1 if it has none.
@abstract func slot_of(unit_id: int) -> int

## The unit in a slot, -1 if the slot is free.
@abstract func id_at(slot: int) -> int

## Units with a slot.
@abstract func count() -> int

## The units with a slot, by slot.
@abstract func ids() -> PackedInt64Array


## Brings the store to the squads' lists: listed units have slots, the rest are dropped.
## Units join only by enrolling (FormationSimulation.spawn_squad), so while the lists hold
## as many units as the store, nothing has left and there's nothing to walk.
func settle(squads: Array) -> void:
	var listed := 0
	for squad in squads:
		listed += squad.units.size()
	if listed == count():
		return
	var kept := {}
	for squad in squads:
		for unit in squad.units:
			kept[unit.id] = true
			enrol(unit.id)
	for unit_id in ids():
		if not kept.has(unit_id):
			drop(unit_id)
			if _records.has(unit_id):
				_records[unit_id].let_go()
			_records.erase(unit_id)
