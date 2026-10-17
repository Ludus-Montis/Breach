extends RefCounted
## Where units stand in the tests (spec 35): one unit store shared by the squads a test
## builds by hand, as a battle's squads share one, and reads and writes where each stands
## and the way it faces through it. A
## test makes one in before_test; a squad from a FormationSimulation reads its own store
## through `of` and `put`. Made on a native field (`UnitPlaces.new(field)`), the store and
## its columns live on that field, as a battle's do, so the field's passes see them.

const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## A unit made without an id gets one from here, as a battle gives each its own.
const FIRST_ID := 1000000

var store: UnitStore


func _init(field: Object = null) -> void:
	store = NativeKernels.unit_store(field)


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


## The way `unit` faces in the shared store (spec 35 step 3).
func facing(unit) -> float:
	return store.columns.value(unit.slot, UnitColumns.BEARING)


## Turns `unit` to face `bearing` in the shared store.
func face(unit, bearing: float) -> void:
	store.columns.put(unit.slot, UnitColumns.BEARING, bearing)


## Puts `unit` at `at` (rank, column) in its squad's frame, in the shared store: enrolled
## there first if it isn't yet (one without an id is given one); returns it.
func set_place(unit, at: Vector2i):
	if unit.id == 0:
		unit.id = FIRST_ID + store.count()
	unit.slot = store.enrol(unit.id)
	UnitPlace.set_at(store.columns, unit.slot, at)
	return unit


## Sets `unit`'s hp to `hp` in the shared store, through UnitHealth (spec 35 step 5b):
## enrolled there first if it isn't yet (one without an id is given one); returns it.
func health(unit, hp: int):
	if unit.id == 0:
		unit.id = FIRST_ID + store.count()
	unit.slot = store.enrol(unit.id)
	UnitHealth.put_one(self, unit, hp)
	return unit


## Sets `unit`'s pace to `speed` in the shared store, and its pace fresh on its record (spec
## 35 step 6c): enrolled there first if it isn't yet; returns it.
func pace(unit, speed: float):
	_enrolled(unit)
	unit.fresh_speed = speed
	store.columns.put(unit.slot, UnitColumns.SPEED, speed)
	return unit


## Sets `unit`'s stamina to `stamina` in the shared store (spec 35 step 6c): enrolled there
## first if it isn't yet; returns it.
func tire(unit, stamina: float):
	_enrolled(unit)
	store.columns.put(unit.slot, UnitColumns.STAMINA, stamina)
	return unit


func _enrolled(unit) -> void:
	if unit.id == 0:
		unit.id = FIRST_ID + store.count()
	unit.slot = store.enrol(unit.id)


## Where `unit` of `squad` stands, in the squad's store.
static func of(squad, unit) -> Vector2:
	return squad.store.columns.value(unit.slot, UnitColumns.POSITION)


## Moves `unit` of `squad` to `point`, in the squad's store.
static func put(squad, unit, point: Vector2) -> void:
	squad.store.columns.put(unit.slot, UnitColumns.POSITION, point)


## The way `unit` of `squad` faces, in the squad's store.
static func bearing_of(squad, unit) -> float:
	return squad.store.columns.value(unit.slot, UnitColumns.BEARING)


## Turns `unit` of `squad` to face `bearing`, in the squad's store.
static func turn(squad, unit, bearing: float) -> void:
	squad.store.columns.put(unit.slot, UnitColumns.BEARING, bearing)


## Where `unit` stands in `store` (a FormationSimulation's).
static func where(store, unit) -> Vector2:
	return store.columns.value(unit.slot, UnitColumns.POSITION)


## Moves `unit` to `point` in `store` (a FormationSimulation's).
static func place(store, unit, point: Vector2) -> void:
	store.columns.put(unit.slot, UnitColumns.POSITION, point)


## Moves `squads` (and `others`, units their lists don't hold, such as bodies laid on the
## ground) onto a native store on `field`, each unit keeping every column's value: a
## battle built under GDScript, its native passes then run on `field` as a battle's run on
## its own. Units may get other slots (a slot is storage only). Returns the new store.
static func onto(field: Object, squads: Array, others: Array = []) -> UnitStore:
	var moved := NativeKernels.unit_store(field)
	var before: UnitStore = squads[0].store
	var units := others.duplicate()
	for squad in squads:
		units.append_array(squad.units)
	for unit in units:
		var values := UnitColumns.BLANKS.keys().map(
			func(name): return before.columns.value(unit.slot, name)
		)
		unit.slot = moved.admit(unit)
		for index in values.size():
			moved.columns.put(unit.slot, UnitColumns.BLANKS.keys()[index], values[index])
	for squad in squads:
		squad.store = moved
	return moved


## A copy of every column of `store`, to put back with `restore`: a test that runs a pass
## twice from one state puts the store back as well as the squads' entries.
static func saved(store: UnitStore) -> Dictionary:
	var out := {}
	for name in UnitColumns.BLANKS:
		out[name] = store.columns.column(name).duplicate()
	return out


## Puts back the columns `saved` copied.
static func restore(store: UnitStore, columns: Dictionary) -> void:
	for name in columns:
		store.columns.commit(name, columns[name].duplicate())
