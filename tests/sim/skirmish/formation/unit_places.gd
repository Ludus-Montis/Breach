extends RefCounted
## Where units stand in the tests (spec 35): one unit store shared by the squads a test
## builds by hand, as a battle's squads share one, and reads and writes through it. A
## test makes one in before_test; a squad from a FormationSimulation reads its own store
## through `of` and `put`.

const UnitStoreGd = preload("res://sim/skirmish/formation/unit_store_gd.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## A unit made without an id gets one from here, as a battle gives each its own.
const FIRST_ID := 1000000

var store := UnitStoreGd.new()


## Enrols `unit` in the shared store, standing at `at` (one without an id is given one);
## returns it.
func stand(unit, at: Vector2):
	if unit.id == 0:
		unit.id = FIRST_ID + store.count()
	unit.slot = store.enrol(unit.id)
	store.columns.put(unit.slot, UnitColumns.POSITION, at)
	return unit


## Where `unit` stands in the shared store.
func at(unit) -> Vector2:
	return store.columns.value(unit.slot, UnitColumns.POSITION)


## Moves `unit` to `point` in the shared store.
func move(unit, point: Vector2) -> void:
	store.columns.put(unit.slot, UnitColumns.POSITION, point)


## Where `unit` of `squad` stands, in the squad's store.
static func of(squad, unit) -> Vector2:
	return squad.store.columns.value(unit.slot, UnitColumns.POSITION)


## Moves `unit` of `squad` to `point`, in the squad's store.
static func put(squad, unit, point: Vector2) -> void:
	squad.store.columns.put(unit.slot, UnitColumns.POSITION, point)


## Where `unit` stands in `store` (a FormationSimulation's).
static func where(store, unit) -> Vector2:
	return store.columns.value(unit.slot, UnitColumns.POSITION)


## Moves `unit` to `point` in `store` (a FormationSimulation's).
static func place(store, unit, point: Vector2) -> void:
	store.columns.put(unit.slot, UnitColumns.POSITION, point)
