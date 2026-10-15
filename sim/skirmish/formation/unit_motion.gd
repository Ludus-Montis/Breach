class_name UnitMotion
extends RefCounted
## How a unit moves and turns on its own (Decisions 95 and 105, spec 27 round 8; model C).
## A unit faces a bearing: an angle in degrees, clockwise from north (0 north, 90 east, 180
## south, 270 west), turned freely. It may move any way, at a pace set by the angle
## between its bearing and its heading - its full speed straight ahead, down to its
## backward pace straight back - while its bearing turns towards its heading, or stays on
## a foe it backs away from, at its turn rate (degrees a second). The bearing is the unit
## store's (UnitColumns.BEARING). Both are unit stats: a
## grem is nimbler than a brute. A unit turning to its places or its foe is exposed, as a
## blow from outside its front is a flank blow (Decision 88). Pure.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001
## The vectors of the bearings 0, 90, 180 and 270, and how far out they're looked up.
const QUARTER_VECTORS := [Vector2(0, -1), Vector2(1, 0), Vector2(0, 1), Vector2(-1, 0)]
const QUARTER_RANGE := 1e6
## How many other bearings' vectors are remembered before starting afresh.
const REMEMBERED := 8192

static var _vectors := {}


## The unit vector of a bearing; exact on the quarter bearings (no 1e-16 residue to tip a
## point across a cell's edge). DetMath's sin and cos are slow in GDScript and most
## bearings asked about are quarter bearings, which are looked up (the same vectors
## DetMath gives them, test_det_math.gd), or a unit's own bearing, asked about many times
## a tick, which is remembered: a pure function's results, so nothing else changes.
static func vector(bearing: float) -> Vector2:
	if fposmod(bearing, 90.0) == 0.0 and absf(bearing) < QUARTER_RANGE:
		return QUARTER_VECTORS[posmod(int(bearing / 90.0), 4)]
	var known = _vectors.get(bearing)
	if known != null:
		return known
	if _vectors.size() >= REMEMBERED:
		_vectors.clear()
	var angle := deg_to_rad(bearing)
	var x := DetMath.sin(angle)
	var y := -DetMath.cos(angle)
	var made := Vector2(x if absf(x) > EPSILON else 0.0, y if absf(y) > EPSILON else 0.0)
	_vectors[bearing] = made
	return made


## The bearing of a squad facing (SquadFrame: 0 north, 1 east, 2 south, 3 west).
static func of_facing(facing: int) -> float:
	return posmod(facing, 4) * 90.0


## The bearing of the way from `from` to `to` (`current` if they coincide).
static func bearing_to(from: Vector2, to: Vector2, current: float) -> float:
	var way := to - from
	if way.length() < EPSILON:
		return current
	return fposmod(rad_to_deg(DetMath.atan2(way.x, -way.y)), 360.0)


## Degrees from bearing `from` to `to` the short way round: positive clockwise, in
## (-180, 180]. An exact about-turn goes clockwise (to the unit's right).
static func _apart(from: float, to: float) -> float:
	var turn := fposmod(to - from, 360.0)
	return turn - 360.0 if turn > 180.0 + EPSILON else turn


## Turns the unit towards `wanted` by what its turn rate allows in `seconds`, its bearing
## in the store's `columns`; true once it faces it.
static func turn(unit: SkirmishUnit, columns: UnitColumns, wanted: float, seconds: float) -> bool:
	var bearing: float = columns.value(unit.slot, UnitColumns.BEARING)
	var left := _apart(bearing, wanted)
	var reach := unit.turn_rate * seconds
	var faces := absf(left) < EPSILON or reach >= absf(left) - EPSILON
	var turned := fposmod(wanted if faces else bearing + signf(left) * reach, 360.0)
	columns.put(unit.slot, UnitColumns.BEARING, turned)
	return faces


## The share of its speed the unit makes moving along `heading` while facing `bearing`: 1
## straight ahead, its backward pace straight back.
static func pace(unit: SkirmishUnit, heading: Vector2, bearing: float) -> float:
	if heading.length() < EPSILON:
		return 1.0
	var cosine := clampf(heading.normalized().dot(vector(bearing)), -1.0, 1.0)
	var off := rad_to_deg(DetMath.acos(cosine)) / 180.0
	return 1.0 - (1.0 - unit.backward_pace) * off


## The unit's next point moving from `at` to `to` with `full_step` cells at full pace, at
## the pace its present bearing allows (it turns separately).
static func move(
	unit: SkirmishUnit, columns: UnitColumns, at: Vector2, to: Vector2, full_step: float
) -> Vector2:
	var bearing: float = columns.value(unit.slot, UnitColumns.BEARING)
	return at.move_toward(to, full_step * pace(unit, to - at, bearing))


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
	var heading := to - at
	var look: Vector2 = face_point if face_point != null else to
	var bearing: float = columns.value(unit.slot, UnitColumns.BEARING)
	turn(unit, columns, bearing_to(at, look, bearing), seconds)
	var turned: float = columns.value(unit.slot, UnitColumns.BEARING)
	return at.move_toward(to, full_step * pace(unit, heading, turned))
