class_name UnitMotion
extends RefCounted
## How a unit moves and turns on its own (Decisions 95 and 105, spec 27 round 8; model C).
## A unit faces a bearing: an angle in degrees, clockwise from north (0 north, 90 east, 180
## south, 270 west), turned freely. It may move any way, at a pace set by the angle
## between its bearing and its heading - its full speed straight ahead, down to its
## backward pace straight back - while its bearing turns towards its heading, or stays on
## a foe it backs away from, at its turn rate (degrees a second). The bearing is the unit
## store's (UnitColumns.BEARING). Both are unit stats: a grem is nimbler than a brute. A
## unit turning to its places or its foe is exposed, as a blow from outside its front is a
## flank blow (Decision 88). A view over the crate's (native/rust/src/motion.rs, spec 39
## step 3b; CrateMoves).

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## The unit vector of a bearing; exact on the quarter bearings (no 1e-16 residue to tip a
## point across a cell's edge).
static func vector(bearing: float) -> Vector2:
	return CrateMoves.core().vector(bearing)


## The bearing of a squad facing (SquadFrame: 0 north, 1 east, 2 south, 3 west).
static func of_facing(facing: int) -> float:
	return posmod(facing, 4) * 90.0


## The bearing of the way from `from` to `to` (`current` if they coincide).
static func bearing_to(from: Vector2, to: Vector2, current: float) -> float:
	return CrateMoves.core().bearing_to(from, to, current)


## Turns the unit towards `wanted` by what its turn rate allows in `seconds`, its bearing
## in the store's `columns`; true once it faces it.
static func turn(unit: SkirmishUnit, columns: UnitColumns, wanted: float, seconds: float) -> bool:
	var turned: PackedFloat64Array = CrateMoves.core().turn(
		CrateMoves.of(unit, columns), wanted, seconds
	)
	columns.put(unit.slot, UnitColumns.BEARING, turned[0])
	return turned[1] != 0.0


## The share of its speed the unit makes moving along `heading` while facing `bearing`: 1
## straight ahead, its backward pace straight back.
static func pace(unit: SkirmishUnit, heading: Vector2, bearing: float) -> float:
	return CrateMoves.core().pace(CrateMoves.of(unit, null), heading, bearing)


## The unit's next point moving from `at` to `to` with `full_step` cells at full pace, at
## the pace its present bearing allows (it turns separately).
static func move(
	unit: SkirmishUnit, columns: UnitColumns, at: Vector2, to: Vector2, full_step: float
) -> Vector2:
	return CrateMoves.core().advance(CrateMoves.of(unit, columns), at, to, full_step)


## The unit's next point walking from `at` to `to` with `full_step` cells at full pace,
## turning as it goes - towards its heading, or to keep facing `face_point` if given.
static func walk(
	unit: SkirmishUnit,
	columns: UnitColumns,
	at: Vector2,
	to: Vector2,
	full_step: float,
	seconds: float,
	face_point = null
) -> Vector2:
	var look: Vector2 = face_point if face_point != null else Vector2.ZERO
	var walked: PackedFloat64Array = CrateMoves.core().walk(
		CrateMoves.of(unit, columns), at, to, full_step, seconds, look, face_point != null
	)
	return CrateMoves.placed(unit, columns, walked)
