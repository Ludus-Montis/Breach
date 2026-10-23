class_name PathMemory
extends RefCounted
## What a pathfinder has seen of the ground (spec 30): the cells its plans have looked at
## within its sight, kept between plans, so a wall it has felt along stays known when it
## turns its back on it and it doesn't walk back to look again. A plan counts a known cell
## as seen (at its real cost, read live from the terrain). One per squad, forgotten once
## the squad is back on its route (a detour that worked is kept as a RoutePatch). The
## crate's plans read and add to it (`known`, CratePaths; spec 39 step 3c). Pure.

## A byte a cell, 1 where it has seen it; empty, or another grid's size, before any.
var known := PackedByteArray()


## Whether it has seen the cell at `index` of a grid of `count` cells.
func knows(index: int, count: int) -> bool:
	if known.size() != count:
		return false
	return known[index] == 1


## Forgets all it has seen: its group is back on its route.
func forget() -> void:
	known = PackedByteArray()


## Remembers seeing the cell at `index` of a grid of `count` cells.
func learn(index: int, count: int) -> void:
	if known.size() != count:
		known.resize(count)
		known.fill(0)
	known[index] = 1
