class_name SquadGeometry
extends RefCounted
## Where squads stand relative to one another in cells (Decisions 74 and 75, spec 27 round
## 1): gaps along a squad's heading, and the point a unit's rank stands at, for the passes
## still in GDScript (joining, ranged combat); whether two lines overlap across a heading or
## face off is the crate's (`geometry.rs`, spec 39 step 4c). On a straight lane these give
## the lane's distance arithmetic in tiles. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)


## How far `to`'s front lies ahead of `from`'s along `from`'s heading, in tiles.
static func gap(from: SkirmishSquad, to: SkirmishSquad) -> float:
	return (to.position - from.position).dot(UnitMotion.vector(from.heading)) / CELLS


## How far a unit of `to` stands by its rank ahead of `from`'s front along its heading, in
## tiles. `from` and `to`: the two squads' frames (StoredSquad.frame, read once a pass);
## `places`: a view of `to`'s store holding its ranks (UnitView.PLACES).
static func unit_gap(from: Array, to: Array, unit: SkirmishUnit, places: UnitView) -> float:
	var rank: int = places.rank[unit.slot]
	var point: Vector2 = to[0] - UnitMotion.vector(to[1]) * rank
	return (point - from[0]).dot(UnitMotion.vector(from[1])) / CELLS
