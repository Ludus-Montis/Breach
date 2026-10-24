class_name CrateMoves
extends RefCounted
## The crate's routes, motion, the one mover and steering (`UnitMoves`, native/rust/src/
## route.rs, motion.rs, mover.rs and steer.rs; spec 39 step 3b), which FormationRoute,
## RoutePatch, UnitMotion, UnitMover, and TerrainStep are views over, and what of
## a unit they pass it. Stateless, so one is shared.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const MOVES_CLASS := "UnitMoves"

static var _core: Object


## The shared one.
static func core() -> Object:
	if _core == null:
		NativeLibrary.load_for(MOVES_CLASS)
		_core = ClassDB.instantiate(MOVES_CLASS)
	return _core


## What of the unit its motion reads: [its bearing in the store's `columns` (0 without
## them), its turn rate, its backward pace].
static func of(unit: SkirmishUnit, columns: UnitColumns) -> PackedFloat64Array:
	var bearing: float = columns.value(unit.slot, UnitColumns.BEARING) if columns != null else 0.0
	return PackedFloat64Array([bearing, unit.turn_rate, unit.backward_pace])


## The point of a crate's [x, y, bearing], its bearing put in the store's `columns`.
static func placed(unit: SkirmishUnit, columns: UnitColumns, walked: PackedFloat64Array) -> Vector2:
	columns.put(unit.slot, UnitColumns.BEARING, walked[2])
	return Vector2(walked[0], walked[1])
