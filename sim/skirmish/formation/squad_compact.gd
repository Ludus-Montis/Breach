class_name SquadCompact
extends RefCounted
## A squad closing up when units fall (Decision 47; SkirmishSquad.compact): its living units
## step up a rank while their whole footprint is clear, reading and writing their places in
## the unit store (spec 35 step 4a).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const PlaceOrder = preload("res://sim/skirmish/formation/place_order.gd")


## Step-up: living units move forward a rank while their whole footprint would be clear,
## until nothing more can move - but only front-preferring units step into the front rank
## (Decision 47). If the whole front has fallen, the squad re-anchors: its foremost rank
## becomes rank 0 and its front_distance moves back to match, so nobody moves on the route.
## Returns the units that stepped up. The places are fetched once, worked on here and the
## ranks committed once (spec 35): only ranks change.
## Each pass looks at the units front first; after the first, a unit is looked at again
## only if a place in one of its columns changed since it was last looked at. The row
## ahead of it lies in its own columns, so any other would find it as held as before: the
## passes move the same units as passes over everyone (spec 35 step 5b).
static func compact(squad: SkirmishSquad) -> Array[SkirmishUnit]:
	var ranks: PackedInt32Array = squad.store.columns.column(UnitColumns.RANK)
	var files: PackedInt32Array = squad.store.columns.column(UnitColumns.COLUMN)
	var living := squad.living()  # no one falls while the squad closes up
	var moved: Array[SkirmishUnit] = []
	var stepped := {}  # unit -> true: those in `moved`
	if living.is_empty():
		return moved
	var movers := {}  # mid-move: the units moves under way are taking to their places
	for swap in squad.swaps:
		for mover in swap["to"]:
			movers[mover] = true
	var held := Held.new(squad, living, ranks, files)
	var looked := {}  # unit -> the time it was last looked at
	var changed := false
	var any := true
	while any:
		any = false
		var order_by_place := _front_first(living, ranks, files)
		for unit: SkirmishUnit in order_by_place:
			if movers.has(unit):
				continue  # mid-move: it takes the place its move is heading for
			var slot := unit.slot
			if looked.has(unit) and not held.changed_since(looked[unit], files[slot], unit):
				continue
			looked[unit] = held.tick()
			var rank := ranks[slot]
			if rank <= 0 or not (rank > 1 or unit.preferred_position == 0):
				continue
			if held.clear_ahead(rank, files[slot], unit.footprint_width):
				held.step_up(rank, files[slot], unit)
				ranks[slot] = rank - 1
				any = true
				changed = true
				if not stepped.has(unit):
					stepped[unit] = true
					moved.append(unit)
	if not living.any(func(u): return ranks[u.slot] == 0):
		var shift: int = living.reduce(func(least, u): return mini(least, ranks[u.slot]), 1 << 30)
		for unit in living:
			ranks[unit.slot] -= shift
		changed = true
		squad.front_distance -= squad.direction * shift * SkirmishSquad.RANK_DEPTH
	if changed:
		squad.store.columns.commit(UnitColumns.RANK, ranks)
	return moved


## PlaceOrder.front_first on the ranks and files being worked on.
static func _front_first(living: Array, ranks: PackedInt32Array, files: PackedInt32Array) -> Array:
	return PlaceOrder.front_first(living, ranks, files)


## The places held while a squad closes up, as a count per place on a grid over every place
## a step-up can look at: how many living units' footprints cover it, plus how many moves
## under way are heading there. A unit's row ahead is never one of its own places, and a
## mover never steps up, so a place is clear to a unit exactly when its count is 0. Each
## column of the grid keeps when a place in it last changed, on a clock `tick` advances.
class Held:
	var counts := PackedInt32Array()
	var changed_at := PackedInt32Array()  # by column of the grid
	var first_rank := 0
	var first_column := 0
	var ranks := 0
	var columns := 0
	var clock := 0

	func _init(
		squad: SkirmishSquad, living: Array, at_rank: PackedInt32Array, at_file: PackedInt32Array
	) -> void:
		var low := Vector2i(0, 1 << 30)  # rank 0: the lowest row a step-up looks at
		var high := Vector2i(-(1 << 30), -(1 << 30))
		for unit in living:
			var slot: int = unit.slot
			low = Vector2i(mini(low.x, at_rank[slot]), mini(low.y, at_file[slot]))
			high = Vector2i(
				maxi(high.x, at_rank[slot] + unit.footprint_depth - 1),
				maxi(high.y, at_file[slot] + unit.footprint_width - 1)
			)
		first_rank = low.x
		first_column = low.y
		ranks = high.x - low.x + 1
		columns = high.y - low.y + 1
		counts.resize(ranks * columns)
		changed_at.resize(columns)
		for unit in living:
			var at := Vector2i(at_rank[unit.slot], at_file[unit.slot])
			_cover(at, unit.footprint_depth, unit.footprint_width)
		for swap in squad.swaps:
			for mover in swap["to"]:
				_cover(swap["to"][mover], mover.footprint_depth, mover.footprint_width)

	## The clock's next time.
	func tick() -> int:
		clock += 1
		return clock

	## True if a place in any of the unit's columns (from `file`) changed after `time`.
	func changed_since(time: int, file: int, unit: SkirmishUnit) -> bool:
		for column in range(file - first_column, file - first_column + unit.footprint_width):
			if changed_at[column] > time:
				return true
		return false

	## True if nothing holds the row ahead of a unit at `rank` and `file`, `width` wide.
	func clear_ahead(rank: int, file: int, width: int) -> bool:
		var row := rank - 1 - first_rank
		if row < 0 or row >= ranks:
			return true
		for column in range(file - first_column, file - first_column + width):
			if counts[row * columns + column] > 0:
				return false
		return true

	## A unit at `rank` and `file` steps up a rank: its footprint here with it.
	func step_up(rank: int, file: int, unit: SkirmishUnit) -> void:
		var now := tick()
		var back := rank + unit.footprint_depth - 1 - first_rank
		var ahead := rank - 1 - first_rank
		for column in range(file - first_column, file - first_column + unit.footprint_width):
			counts[back * columns + column] -= 1
			counts[ahead * columns + column] += 1
			changed_at[column] = now

	func _cover(at: Vector2i, depth: int, width: int) -> void:
		for rank in range(at.x - first_rank, at.x - first_rank + depth):
			for column in range(at.y - first_column, at.y - first_column + width):
				if rank >= 0 and rank < ranks and column >= 0 and column < columns:
					counts[rank * columns + column] += 1
