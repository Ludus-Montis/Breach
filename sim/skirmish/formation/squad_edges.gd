class_name SquadEdges
extends RefCounted
## A squad's four edges (Decision 78, spec 27 round 2): front, right, rear and left,
## relative to its heading (clockwise, as headings are). Which edge an attacker meets, which
## units stand on an edge (the outermost on that side, so when one falls the next inward
## is the edge: step-up inward is implicit), and how far an attacker's front is from a
## squad's near face. On a turned frame each edge is a quarter of the way round, its
## footprints turned with it. Pure; cells unless stated.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const FRONT := 0
const RIGHT := 1
const REAR := 2
const LEFT := 3
const EPSILON := 0.000001


## The cells the squad's living units cover.
static func bounds(squad: SkirmishSquad) -> Rect2:
	var area := Rect2()
	var first := true
	for unit in squad.living():
		var rect := _rect(squad, unit)
		area = rect if first else area.merge(rect)
		first = false
	return area


## The edge of `victim` an attacker heading `attacker_heading` (degrees) meets with its
## front: the quarter of the victim's frame it comes from (on the line between two, the one
## clockwise of it).
static func edge_hit(victim: SkirmishSquad, attacker_heading: float) -> int:
	var from := fposmod(attacker_heading + 180.0 - victim.heading, 360.0)
	return posmod(roundi(from / 90.0), 4)


## The units on an edge: those with no living unit of their squad beyond them that way.
static func edge_units(squad: SkirmishSquad, edge: int) -> Array[SkirmishUnit]:
	var outward := UnitMotion.vector(squad.heading + edge * 90.0)
	var across := SquadFrame.lateral_axis(squad.heading + edge * 90.0)
	var living := squad.living()
	var out: Array[SkirmishUnit] = []
	for unit in living:
		var points := _corners(squad, unit)
		var span := SquadFrame.extent(points, across)
		var reach := SquadFrame.extent(points, outward).y
		var covered := false
		for other in living:
			if other == unit:
				continue
			var other_points := _corners(squad, other)
			var other_span := SquadFrame.extent(other_points, across)
			var overlap := minf(span.y, other_span.y) - maxf(span.x, other_span.x)
			if overlap > EPSILON and SquadFrame.extent(other_points, outward).y > reach + EPSILON:
				covered = true
				break
		if not covered:
			out.append(unit)
	return out


## How far the squad's living footprints reach along `way`: (least, most).
static func reach(squad: SkirmishSquad, way: Vector2) -> Vector2:
	return SquadFrame.extent(_all_corners(squad), way)


## How far `to`'s near face lies ahead of `from`'s front along `from`'s heading, in tiles.
static func face_gap(from: SkirmishSquad, to: SkirmishSquad) -> float:
	var ahead := UnitMotion.vector(from.heading)
	var near := reach(to, ahead).x
	return (near - from.position.dot(ahead)) / MapLayoutDef.CELLS_PER_TILE


## True if `from`'s line overlaps `to`'s footprints across `from`'s heading.
static func overlap_across(from: SkirmishSquad, to: SkirmishSquad) -> bool:
	var axis := SquadFrame.lateral_axis(from.heading)
	var mine := Vector2(INF, -INF)
	for unit in from.living():
		var span := from.lateral_span(unit, axis)
		mine = Vector2(minf(mine.x, span.x), maxf(mine.y, span.y))
	var theirs := reach(to, axis)
	return minf(mine.y, theirs.y) - maxf(mine.x, theirs.x) > EPSILON


static func _rect(squad: SkirmishSquad, unit: SkirmishUnit) -> Rect2:
	return SquadFrame.unit_rect(
		squad.position,
		squad.heading,
		squad.width,
		squad.centre_shift,
		unit,
		UnitPlace.of(squad, unit)
	)


static func _corners(squad: SkirmishSquad, unit: SkirmishUnit) -> PackedVector2Array:
	var at := UnitPlace.of(squad, unit)
	return SquadFrame.corners(
		squad.position, squad.heading, squad.width, squad.centre_shift, unit, at
	)


## The corners of all the squad's living units' footprints.
static func _all_corners(squad: SkirmishSquad) -> PackedVector2Array:
	var points := PackedVector2Array()
	for unit in squad.living():
		points.append_array(_corners(squad, unit))
	return points
