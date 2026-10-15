class_name UnitShuffle
extends RefCounted
## Taking a place in a formation (Decision 92: a turn is a re-form; spec 27 round 11): a
## unit walking to its place arrives facing the formation's way by whichever is quicker -
## shuffling there facing that way, at the pace its bearing allows (UnitMotion), or turning
## to walk, walking at full pace and turning back. So a unit stepping back a cell or two
## keeps its face to the enemy rather than spinning round. Pure.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

const EPSILON := 0.000001


## The point the unit should look towards walking from `at` to its place `to`, to arrive
## facing `bearing` soonest, facing `facing` now; `speed` is its full speed in cells a
## second.
static func look(
	unit: SkirmishUnit, at: Vector2, facing: float, to: Vector2, bearing: float, speed: float
) -> Vector2:
	var way := to - at
	var ahead := at + UnitMotion.vector(bearing)
	if way.length() < EPSILON or speed < EPSILON:
		return ahead
	var heading := UnitMotion.bearing_to(at, to, facing)
	var facing_pace := maxf(UnitMotion.pace(unit, way, bearing), EPSILON)
	var shuffling := _turning(unit, facing, bearing) + way.length() / (speed * facing_pace)
	var walking := (
		_turning(unit, facing, heading) + way.length() / speed + _turning(unit, heading, bearing)
	)
	return ahead if shuffling <= walking else to


## Seconds the unit takes to turn from one bearing to another.
static func _turning(unit: SkirmishUnit, from: float, to: float) -> float:
	var cosine := clampf(UnitMotion.vector(from).dot(UnitMotion.vector(to)), -1.0, 1.0)
	return rad_to_deg(DetMath.acos(cosine)) / maxf(unit.turn_rate, EPSILON)
