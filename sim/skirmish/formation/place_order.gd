class_name PlaceOrder
extends RefCounted
## Units front first by their places in a squad's frame (rank, then column), as compact and
## a re-form (FormationShuffle) look through them: sorted on whole-number keys rather than
## by a comparison called for every pair. Two units on one place (which a frame never
## holds) are sorted as a comparison of ranks and columns would sort them, so the order is
## always the same as that sort's. Pure.

## Ranks and columns this far either side of 0 fit a key; any beyond are sorted as pairs.
const LIMIT := 1 << 20


## `units`, front rank first, each rank left to right, by their places in `ranks` and
## `files` (the unit store's RANK and COLUMN, by slot: a view's, UnitView.PLACES).
static func front_first(units: Array, ranks: PackedInt32Array, files: PackedInt32Array) -> Array:
	var count := units.size()
	var keys := PackedInt64Array()
	keys.resize(count)
	for index in range(count):
		var slot: int = units[index].slot
		if absi(ranks[slot]) >= LIMIT or absi(files[slot]) >= LIMIT:
			return _by_pairs(units, ranks, files)
		var place: int = (ranks[slot] + LIMIT) * (2 * LIMIT) + files[slot] + LIMIT
		keys[index] = place * count + index
	keys.sort()
	var out := []
	out.resize(count)
	for index in range(count):
		out[index] = units[keys[index] % count]
		if index > 0 and keys[index] / count == keys[index - 1] / count:
			return _by_pairs(units, ranks, files)  # one place held twice: as the comparison sorts them
	return out


static func _by_pairs(units: Array, ranks: PackedInt32Array, files: PackedInt32Array) -> Array:
	var out := units.duplicate()
	out.sort_custom(
		func(a, b):
			var ahead := ranks[a.slot] < ranks[b.slot]
			return ahead or (ranks[a.slot] == ranks[b.slot] and files[a.slot] < files[b.slot])
	)
	return out
