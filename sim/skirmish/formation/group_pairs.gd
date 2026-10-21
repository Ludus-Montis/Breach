class_name GroupPairs
extends RefCounted
## The pairs of groups standing near enough that forming up looks at them (spec 38 step 1a,
## Decision 150): found through a grid of the groups' positions rather than by trying every
## pair. A pair is a candidate when its positions are within the two groups' reaches and
## `join` of each other; the grid's cells are as wide as the largest such distance, so every
## candidate lies in a cell next to the other's. Pairs come in the groups' own list order,
## as trying every pair gave them. Pure.


## [[i, j], ...] with i < j: indices into `groups` of each candidate pair, i ascending and,
## for each i, j ascending. `reaches[k]` is how far group k's bodies may lie from its
## position.
static func near(groups: Array, reaches: Array, join: float) -> Array:
	var cell := join
	for reach: float in reaches:
		cell = maxf(cell, 2.0 * reach + join)
	var cells := {}  # Vector2i -> [indices]
	for index in groups.size():
		var key := _cell(groups[index].position, cell)
		if not cells.has(key):
			cells[key] = []
		cells[key].append(index)
	var pairs := []
	for i in groups.size():
		var here := _cell(groups[i].position, cell)
		var found: Array = []
		for dx in range(-1, 2):
			for dy in range(-1, 2):
				for j: int in cells.get(here + Vector2i(dx, dy), []):
					var apart: float = groups[i].position.distance_to(groups[j].position)
					if j > i and apart <= reaches[i] + reaches[j] + join:
						found.append(j)
		found.sort()
		for j in found:
			pairs.append([i, j])
	return pairs


static func _cell(point: Vector2, cell: float) -> Vector2i:
	return Vector2i(floori(point.x / cell), floori(point.y / cell))
