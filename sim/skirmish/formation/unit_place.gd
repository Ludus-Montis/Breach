class_name UnitPlace
extends RefCounted
## A unit's place in its squad's frame (spec 35 step 4a): its rank back from the front and
## its column from the squad's left, held in the unit store's RANK and COLUMN columns.
## Scattered use: a pass over many units fetches the two columns once instead.


## The unit's place, (rank, column), in its squad's store.
static func of(squad, unit) -> Vector2i:
	return at(squad.store.columns, unit.slot)


## Puts the unit at `place` (rank, column) in its squad's store.
static func put(squad, unit, place: Vector2i) -> void:
	set_at(squad.store.columns, unit.slot, place)


## The unit's rank in its squad's frame, 0 = the front rank.
static func rank(squad, unit) -> int:
	return squad.store.columns.value(unit.slot, UnitColumns.RANK)


## The place, (rank, column), held in `slot` of `columns`.
static func at(columns: UnitColumns, slot: int) -> Vector2i:
	return Vector2i(columns.value(slot, UnitColumns.RANK), columns.value(slot, UnitColumns.COLUMN))


## Sets the place held in `slot` of `columns` to `place` (rank, column).
static func set_at(columns: UnitColumns, slot: int, place: Vector2i) -> void:
	columns.put(slot, UnitColumns.RANK, place.x)
	columns.put(slot, UnitColumns.COLUMN, place.y)
