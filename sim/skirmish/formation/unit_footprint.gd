class_name UnitFootprint
extends RefCounted
## A unit's footprint (Decision 102, spec 30): a rectangle its own width by depth - a grem
## 1 x 1, a brute 2 x 2 - centred where it stands and turned to its bearing, its depth
## along it and its width across. No two footprints overlap; two units touch when their
## footprints come within reach_contact (BattleTuning). Pure; cells.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")


## The footprint's corners for a unit standing at `at` facing `bearing`.
static func corners(unit: SkirmishUnit, at: Vector2, bearing: float) -> PackedVector2Array:
	var ahead := UnitMotion.vector(bearing) * unit.footprint_depth / 2.0
	var across := UnitMotion.vector(bearing + 90.0) * unit.footprint_width / 2.0
	return PackedVector2Array(
		[at + ahead + across, at + ahead - across, at - ahead - across, at - ahead + across]
	)


## How far apart two footprints are, from their corners (0 where they touch or overlap):
## the widest gap along any of their sides' axes. For footprints square to each other it is
## the gap between their boxes, a corner's gap counting as the wider of its two.
static func gap(a: PackedVector2Array, b: PackedVector2Array) -> float:
	var widest := 0.0
	for axis in _axes(a) + _axes(b):
		var mine := _span(a, axis)
		var theirs := _span(b, axis)
		widest = maxf(widest, maxf(theirs.x - mine.y, mine.x - theirs.y))
	return widest


## How deep two footprints overlap, and which way to push `a` out of `b` by the least:
## [depth, unit direction]. Depth 0 if they don't overlap.
static func overlap(a: PackedVector2Array, b: PackedVector2Array) -> Array:
	var least := INF
	var way := Vector2.ZERO
	for axis in _axes(a) + _axes(b):
		var mine := _span(a, axis)
		var theirs := _span(b, axis)
		var depth := minf(mine.y - theirs.x, theirs.y - mine.x)
		if depth <= 0.0:
			return [0.0, Vector2.ZERO]
		if depth < least:
			least = depth
			var outward := (mine.x + mine.y) - (theirs.x + theirs.y)
			way = axis if outward >= 0.0 else -axis
	return [least, way]


## The two axes of a footprint's sides.
static func _axes(corners_of: PackedVector2Array) -> Array:
	return [
		(corners_of[1] - corners_of[0]).normalized(),
		(corners_of[2] - corners_of[1]).normalized(),
	]


## [least, most] of the corners projected on `axis`.
static func _span(corners_of: PackedVector2Array, axis: Vector2) -> Vector2:
	var out := Vector2(INF, -INF)
	for corner in corners_of:
		var along := corner.dot(axis)
		out = Vector2(minf(out.x, along), maxf(out.y, along))
	return out
