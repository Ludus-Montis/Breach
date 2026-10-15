@abstract class_name UnitStore
extends RefCounted
## The battle's unit store (specs/35-unit-store.md, Decision 142): every unit in a list the
## simulation or the digest reads has a dense slot, given when it enrols and freed when it
## is in no list. Free slots are reused lowest first. A slot is storage only: no loop runs
## in slot order and no tie is broken by slot (Decision 97). Two backends keep one
## contract (test_unit_store.gd): UnitStoreGd, the reference, and UnitStoreNative on the
## battle's BodyField; NativeKernels.unit_store picks one, as it picks the engine. Columns
## of per-tick state are in `columns` (UnitColumns), indexed by slot; a unit's slot is on
## its record (SkirmishUnit.slot) and reached through its squad (SkirmishSquad.store).
## On the native backend the columns live on the field (UnitColumnsNative), and a native
## pass runs on the field of the squads' own store (`field`).

const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

var columns: UnitColumns = UnitColumns.new()


## The native BodyField the store and its columns live on; null for the GDScript store.
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
