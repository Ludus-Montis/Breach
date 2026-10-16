class_name SkirmishRoute
extends RefCounted
## Maps a skirmish unit's distance along its route (in cells) to a point on the route
## polyline (the link's route from
## MapLayoutView.route_points), per specs/21-realtime-skirmish-feel-test.md. Pure.


static func length_cells(points: PackedVector2Array, cell_size: float) -> float:
	var total := 0.0
	for i in range(1, points.size()):
		total += points[i - 1].distance_to(points[i])
	return total / cell_size


static func point_at(
	points: PackedVector2Array, distance_cells: float, cell_size: float
) -> Vector2:
	if points.is_empty():
		return Vector2.ZERO
	var remaining := maxf(distance_cells, 0.0) * cell_size
	for i in range(1, points.size()):
		var segment := points[i - 1].distance_to(points[i])
		if remaining <= segment:
			return points[i - 1].lerp(points[i], remaining / segment if segment > 0.0 else 0.0)
		remaining -= segment
	return points[-1]


## The unit normal of the route segment under a distance (the travel direction turned a
## quarter to the right in screen space): formation columns are laid out along it.
static func normal_at(
	points: PackedVector2Array, distance_cells: float, cell_size: float
) -> Vector2:
	if points.size() < 2:
		return Vector2(0, 1)
	var remaining := maxf(distance_cells, 0.0) * cell_size
	var segment_index := points.size() - 1
	for i in range(1, points.size()):
		var segment := points[i - 1].distance_to(points[i])
		if remaining <= segment:
			segment_index = i
			break
		remaining -= segment
	var along := (points[segment_index] - points[segment_index - 1]).normalized()
	return Vector2(-along.y, along.x)
