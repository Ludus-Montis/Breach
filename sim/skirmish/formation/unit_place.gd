class_name UnitPlace
extends RefCounted
## A unit's place in its squad's frame (spec 35 step 4a): its rank back from the front and
## its column from the squad's left, held in the unit store's RANK and COLUMN columns.
## Scattered use; a pass over many units fetches the two columns once instead (`table`) and
## reads each unit's place from them (`read`): under Rust each scattered read is a call
## into the field.


## The unit's place, (rank, column), in its squad's store.
static func of(squad, unit) -> Vector2i:
	return at(squad.store.columns, unit.slot)


## Puts the unit at `place` (rank, column) in its squad's store.
static func put(squad, unit, place: Vector2i) -> void:
	set_at(squad.store.columns, unit.slot, place)


## The unit's rank in its squad's frame, 0 = the front rank.
static func rank(squad, unit) -> int:
	return squad.store.columns.value(unit.slot, UnitColumns.RANK)


## The squad's store's ranks and columns, fetched once: [ranks, columns], by slot. A copy as
## of now: a pass that moves units fetches again.
static func table(squad) -> Array:
	var columns: UnitColumns = squad.store.columns
	return [columns.column(UnitColumns.RANK), columns.column(UnitColumns.COLUMN)]


## The unit's place, (rank, column), from `places` (table(squad)); read from the store if
## `places` is empty.
static func read(places: Array, squad, unit) -> Vector2i:
	if places.is_empty():
		return of(squad, unit)
	return Vector2i(places[0][unit.slot], places[1][unit.slot])


## The place, (rank, column), held in `slot` of `columns`.
static func at(columns: UnitColumns, slot: int) -> Vector2i:
	return Vector2i(columns.value(slot, UnitColumns.RANK), columns.value(slot, UnitColumns.COLUMN))


## Sets the place held in `slot` of `columns` to `place` (rank, column).
static func set_at(columns: UnitColumns, slot: int, place: Vector2i) -> void:
	columns.put(slot, UnitColumns.RANK, place.x)
	columns.put(slot, UnitColumns.COLUMN, place.y)
