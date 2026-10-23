class_name FormationRoute
extends RefCounted
## A route as a path of cells across the map (Decision 75, specs/27-formations-in-2d.md):
## waypoints in cells, walked by distance along them. A squad keeps its distance along its
## route and takes its 2D place and heading from it (Decision 74). The corridor is how far
## either side of the centre line squads may spread. A view over the crate's route
## (native/rust/src/route.rs, spec 39 step 3b); distances are in cells.

## Cells either side of the centre line squads may use (the lane's combat width, halved).
var corridor_half_width := 0.0

var _points := PackedVector2Array()


## A route running east from the origin: the one-lane feel test's lane.
static func straight(length: float, half_width: float = 0.0) -> FormationRoute:
	return FormationRoute.new(PackedVector2Array([Vector2.ZERO, Vector2(length, 0.0)]), half_width)


func _init(points: PackedVector2Array, half_width: float = 0.0) -> void:
	_points = points
	corridor_half_width = half_width


## Its waypoints, in cells.
func points() -> PackedVector2Array:
	return _points


func length_cells() -> float:
	return CrateMoves.core().route_length(_points)


## The point at a distance along the route, clamped to its ends.
func point_at(distance: float) -> Vector2:
	return CrateMoves.core().point_at(_points, distance)


## The unit direction of travel at a distance; at a bend, the leg leaving it.
func heading_at(distance: float) -> Vector2:
	return CrateMoves.core().heading_at(_points, distance)


## The distance along the route of the point on it nearest `point`.
func distance_of(point: Vector2) -> float:
	return CrateMoves.core().distance_of(_points, point)
