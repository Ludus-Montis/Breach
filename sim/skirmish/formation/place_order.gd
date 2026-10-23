class_name PlaceOrder
extends RefCounted
## Units front first by their places in a squad's frame (rank, then column), as compact and
## a re-form (FormationShuffle) look through them. Two units on one place (which a frame
## never holds) keep their order. Pure. A view over the crate's
## (native/rust/src/march/shuffle.rs, spec 39 step 3d).


## `units`, front rank first, each rank left to right, by their places in `ranks` and
## `files` (the unit store's RANK and COLUMN, by slot: a view's, UnitView.PLACES).
static func front_first(units: Array, ranks: PackedInt32Array, files: PackedInt32Array) -> Array:
	var unit_ranks := PackedInt32Array()
	var unit_files := PackedInt32Array()
	for unit in units:
		unit_ranks.append(ranks[unit.slot])
		unit_files.append(files[unit.slot])
	var order: PackedInt32Array = CrateMarch.core().front_first(unit_ranks, unit_files)
	var out := []
	out.resize(units.size())
	for index in order.size():
		out[index] = units[order[index]]
	return out
