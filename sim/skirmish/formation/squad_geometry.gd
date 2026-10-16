class_name SquadGeometry
extends RefCounted
## Where squads stand relative to one another in cells (Decisions 74 and 75, spec 27 round
## 1): gaps along a squad's heading, whether two lines overlap across it, and the point a
## unit's rank stands at. On a straight lane these give the lane's distance arithmetic in
## tiles. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)
const EPSILON := 0.000001
## Degrees off parallel two squads' headings may be and still share an axis.
const PARALLEL := 45.0
## Degrees apart two squads' headings must be for them to face each other's way.
const FACING_OFF := 135.0


## How far `to`'s front lies ahead of `from`'s along `from`'s heading, in tiles.
static func gap(from: SkirmishSquad, to: SkirmishSquad) -> float:
	return (to.position - from.position).dot(UnitMotion.vector(from.heading)) / CELLS


## How far a unit's rank stands ahead of `from`'s front along its heading, in tiles.
## `places`: a view of `to`'s store holding its ranks (UnitView.PLACES).
static func unit_gap(
	from: SkirmishSquad, to: SkirmishSquad, unit: SkirmishUnit, places: UnitView
) -> float:
	var rank: int = places.rank[unit.slot]
	var point := to.position - UnitMotion.vector(to.heading) * rank
	return (point - from.position).dot(UnitMotion.vector(from.heading)) / CELLS


## True if the two squads' lines overlap across `from`'s heading, the two headed along
## one axis (within 45 degrees of parallel, either way).
static func overlaps(from: SkirmishSquad, to: SkirmishSquad) -> bool:
	var apart := fposmod(from.heading - to.heading, 180.0)
	if apart > PARALLEL + EPSILON and apart < 180.0 - PARALLEL - EPSILON:
		return false
	var axis := SquadFrame.lateral_axis(from.heading)
	var mine := lateral(from, axis)
	var theirs := lateral(to, axis)
	return minf(mine.y, theirs.y) - maxf(mine.x, theirs.x) > EPSILON


## True if the two squads face each other's way, their headings at least FACING_OFF degrees
## apart: only these meet front to front.
static func facing_off(from: SkirmishSquad, to: SkirmishSquad) -> bool:
	var apart := absf(fposmod(from.heading - to.heading + 180.0, 360.0) - 180.0)
	return apart > FACING_OFF - EPSILON


## The squad's extent along `axis` (its own lateral axis if none), from its living units'
## spans (empty: none).
static func lateral(squad: SkirmishSquad, axis := Vector2.ZERO) -> Vector2:
	var extent := Vector2(INF, -INF)
	var places := UnitView.read(squad, UnitView.PLACES)  # read once a pass (Decision 146)
	for unit in squad.living():
		var span := squad.lateral_span(unit, axis, UnitPlace.in_view(places, unit))
		extent = Vector2(minf(extent.x, span.x), maxf(extent.y, span.y))
	return extent
