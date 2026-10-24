class_name BodyGrid
extends RefCounted
## Finding what is near a point without looking at everything (spec 30 round 3): points -
## bodies, slots, claims - bucketed by square cells CELL across, built once for a snapshot.
## A query gives the indices of every point in the cells it could reach, in the order the
## points were given, so a search over them keeps the list's order wherever it would have
## (a superset of what qualifies, never a different order: outcomes stay the same, Decision
## 97). Searches outward go ring by ring of cells, each no nearer than the one before.
## Pure; cells.

## Cells a bucket spans: about the widest body, so a touch is a neighbouring bucket away.
const CELL := 2.0
## Cells added to a search's reach: far above float rounding, so no point is missed.
const MARGIN := 0.01


## {"cells": {Vector2i: [index, ...]}, "low", "high": the occupied cells' corners}.
static func build(points: Array) -> Dictionary:
	var cells := {}
	var low := Vector2i(1 << 30, 1 << 30)
	var high := -low
	for index in range(points.size()):
		var cell := cell_of(points[index])
		var bucket: Array = cells.get(cell, [])
		if bucket.is_empty():
			cells[cell] = bucket
			low = low.min(cell)
			high = high.max(cell)
		bucket.append(index)
	return {"cells": cells, "low": low, "high": high}


## A grid over bodies ([[point, radius, unit], ...]), with "widest": the
## largest radius among them, so a search for any body touching a point knows how far to
## look.
static func of_bodies(bodies: Array) -> Dictionary:
	var grid := build(bodies.map(func(body): return body[0]))
	var widest := 0.0
	for body in bodies:
		widest = maxf(widest, body[1])
	grid["widest"] = widest
	return grid


## The cell holding `point`.
static func cell_of(point: Vector2) -> Vector2i:
	return Vector2i(floori(point.x / CELL), floori(point.y / CELL))


## Indices of the points in every cell within `reach` of `point` (and some just beyond),
## in ascending order; with `end`, of the way from `point` to `end`.
static func near(grid: Dictionary, point: Vector2, reach: float, end := point) -> Array:
	var low: Vector2i = grid["low"]
	var high: Vector2i = grid["high"]
	var from := cell_of(point.min(end) - Vector2(reach, reach)).max(low)
	var to := cell_of(point.max(end) + Vector2(reach, reach)).min(high)
	var cells: Dictionary = grid["cells"]
	var out := []
	var buckets := 0
	for y in range(from.y, to.y + 1):
		for x in range(from.x, to.x + 1):
			var bucket = cells.get(Vector2i(x, y))
			if bucket != null:
				out.append_array(bucket)
				buckets += 1
	if buckets > 1:
		out.sort()
	return out


## Indices of the points in the cells exactly `ring_number` cells (Chebyshev) from
## `centre`; no point among them lies nearer a point in `centre` than floor_of(ring). With
## `point` and `within`, only the cells that come within `within` of `point`.
static func ring(
	grid: Dictionary, centre: Vector2i, ring_number: int, point := Vector2.ZERO, within := INF
) -> Array:
	var cells: Dictionary = grid["cells"]
	var reach := within + MARGIN
	var low: Vector2i = grid["low"].max(centre - Vector2i.ONE * ring_number)
	var high: Vector2i = grid["high"].min(centre + Vector2i.ONE * ring_number)
	if reach < INF:
		low = low.max(cell_of(point - Vector2(reach, reach)))
		high = high.min(cell_of(point + Vector2(reach, reach)))
	var out := []
	for y in range(low.y, high.y + 1):
		var across := maxf(maxf(y * CELL - point.y, point.y - (y + 1) * CELL), 0.0)
		var columns: Array = [centre.x - ring_number, centre.x + ring_number]
		if absi(y - centre.y) == ring_number:
			columns = range(low.x, high.x + 1)
		for x in columns:
			if x < low.x or x > high.x:
				continue
			var along := maxf(maxf(x * CELL - point.x, point.x - (x + 1) * CELL), 0.0)
			if along * along + across * across > reach * reach:
				continue
			var bucket = cells.get(Vector2i(x, y))
			if bucket != null:
				out.append_array(bucket)
	return out


## The least distance from a point in the centre cell to any point in ring `ring_number`.
static func floor_of(ring_number: int) -> float:
	return maxf(0.0, (ring_number - 1) * CELL)


## Vector2i(first, last): no ring round `centre` before the first or after the last holds
## an occupied cell (last < first if none is). The first comes from each cell's distance
## in rings to the nearest occupied one, worked out once a grid, the first time it is asked.
## With `point` and `within`, only the rings holding cells within `within` of `point`.
static func ring_span(
	grid: Dictionary, centre: Vector2i, point := Vector2.ZERO, within := INF
) -> Vector2i:
	if grid["cells"].is_empty():
		return Vector2i(0, -1)
	if within < INF:
		var reach := Vector2.ONE * (within + MARGIN)
		var box_low: Vector2i = cell_of(point - reach).max(grid["low"])
		var box_high: Vector2i = cell_of(point + reach).min(grid["high"])
		if box_low.x > box_high.x or box_low.y > box_high.y:
			return Vector2i(0, -1)
		var span := ring_span(grid, centre)
		var short := (box_low - centre).max(centre - box_high).max(Vector2i.ZERO)
		var long := (box_low - centre).abs().max((box_high - centre).abs())
		return Vector2i(maxi(span.x, maxi(short.x, short.y)), mini(span.y, maxi(long.x, long.y)))
	var low: Vector2i = grid["low"]
	var high: Vector2i = grid["high"]
	var far := (low - centre).abs().max((high - centre).abs())
	var inside := centre.clamp(low, high)
	var outside := (centre - inside).abs()
	if not grid.has("rings"):
		grid["rings"] = _rings(grid)
	var width := high.x - low.x + 1
	var rings: PackedInt32Array = grid["rings"]
	var past := maxi(outside.x, outside.y)
	var first := rings[(inside.y - low.y) * width + inside.x - low.x] - past
	return Vector2i(maxi(maxi(first, past), 0), maxi(far.x, far.y))


## For each cell of the grid's box, row by row, its distance in rings (Chebyshev) to the
## nearest occupied cell: two sweeps of the box.
static func _rings(grid: Dictionary) -> PackedInt32Array:
	var low: Vector2i = grid["low"]
	var size: Vector2i = grid["high"] - low + Vector2i.ONE
	var out := PackedInt32Array()
	out.resize(size.x * size.y)
	out.fill(size.x + size.y)
	for cell in grid["cells"]:
		out[(cell.y - low.y) * size.x + cell.x - low.x] = 0
	for sweep in [1, -1]:
		var rows := range(size.y) if sweep == 1 else range(size.y - 1, -1, -1)
		var columns := range(size.x) if sweep == 1 else range(size.x - 1, -1, -1)
		for y in rows:
			for x in columns:
				var least: int = out[y * size.x + x]
				for step in [Vector2i(-1, 0), Vector2i(-1, -1), Vector2i(0, -1), Vector2i(1, -1)]:
					var from: Vector2i = Vector2i(x, y) + step * sweep
					if from.x >= 0 and from.x < size.x and from.y >= 0 and from.y < size.y:
						least = mini(least, out[from.y * size.x + from.x] + 1)
				out[y * size.x + x] = least
	return out
