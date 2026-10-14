class_name SquadFrame
extends RefCounted
## A squad's frame in cells (Decisions 74 and 105, specs/27-formations-in-2d.md and
## 30-continuous-positioning.md): its position is the centre of its front edge, and it
## faces a heading (degrees clockwise from north), or one of four ways for what still
## reckons in those. A unit's (rank, column) is turned to it, never mirrored, so the squad's
## left stays its left. Screen convention: x east, y south. Facings run clockwise. Pure.

const NORTH := 0
const EAST := 1
const SOUTH := 2
const WEST := 3

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")

const _FORWARD := [Vector2(0, -1), Vector2(1, 0), Vector2(0, 1), Vector2(-1, 0)]


static func forward(facing: int) -> Vector2:
	return _FORWARD[posmod(facing, 4)]


static func opposite(facing: int) -> int:
	return posmod(facing + 2, 4)


## The corners of a unit's footprint in a frame at `anchor` turned to `heading` (degrees):
## its columns from the squad's left (centred on the front centre, moved by centre_shift),
## its ranks back from the front.
static func corners(
	anchor: Vector2, heading: float, width: int, centre_shift: float, unit: SkirmishUnit
) -> PackedVector2Array:
	var ahead := UnitMotion.vector(heading)
	var right := -ahead.orthogonal()
	var near := anchor + right * (unit.column - width / 2.0 + centre_shift) - ahead * unit.rank
	var wide := right * unit.footprint_width
	var deep := -ahead * unit.footprint_depth
	return PackedVector2Array([near, near + wide, near + wide + deep, near + deep])


## The cells a unit's footprint spans in a frame turned to `heading`: the box round its
## corners (on a quarter heading, the footprint itself).
static func unit_rect(
	anchor: Vector2, heading: float, width: int, centre_shift: float, unit: SkirmishUnit
) -> Rect2:
	var points := corners(anchor, heading, width, centre_shift, unit)
	var area := Rect2(points[0], Vector2.ZERO)
	for point in points:
		area = area.expand(point)
	return area


## The centre of a unit's place in a frame at `anchor` turned to `heading` (degrees): its
## columns from the squad's left (centred on the front centre, moved by centre_shift), its
## ranks back from the front.
static func place(
	anchor: Vector2, heading: float, width: int, centre_shift: float, unit: SkirmishUnit
) -> Vector2:
	var ahead := UnitMotion.vector(heading)
	var across := unit.column - width / 2.0 + centre_shift + unit.footprint_width / 2.0
	var back := unit.rank + unit.footprint_depth / 2.0
	return anchor + ahead.orthogonal() * -across - ahead * back


## place()'s frame worked out once, for many units of one squad: [anchor, ahead, ahead's
## orthogonal, half the width, centre_shift]. place_in() then gives place()'s point, bit for
## bit, without turning the heading again for each unit.
static func basis(anchor: Vector2, heading: float, width: int, centre_shift: float) -> Array:
	var ahead := UnitMotion.vector(heading)
	return [anchor, ahead, ahead.orthogonal(), width / 2.0, centre_shift]


## place() in a frame from basis().
static func place_in(frame: Array, unit: SkirmishUnit) -> Vector2:
	var across: float = unit.column - frame[3] + frame[4] + unit.footprint_width / 2.0
	var back := unit.rank + unit.footprint_depth / 2.0
	return frame[0] + frame[2] * -across - frame[1] * back


## The world axis a squad's columns run along at `heading`: its right hand, turned to
## point east of north-south (or south, facing east or west), so two squads across one axis
## measure their spans the same way round.
static func lateral_axis(heading: float) -> Vector2:
	return UnitMotion.vector(fposmod(heading, 180.0) + 90.0)


## How far `points` reach along `axis`: (least, most).
static func extent(points: PackedVector2Array, axis: Vector2) -> Vector2:
	var reach := Vector2(INF, -INF)
	for point in points:
		var along := point.dot(axis)
		reach = Vector2(minf(reach.x, along), maxf(reach.y, along))
	return reach
