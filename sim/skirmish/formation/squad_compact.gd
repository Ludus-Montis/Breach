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
## Returns the units that stepped up. The places are fetched once (UnitPlace.table), worked
## on here and the ranks committed once (spec 35): only ranks change.
static func compact(squad: SkirmishSquad) -> Array[SkirmishUnit]:
	var places := UnitPlace.table(squad)
	var moved: Array[SkirmishUnit] = []
	var stepped := {}  # unit -> true: those in `moved`
	var cells := _cells(squad, places)  # who stands where, kept as units step up
	var heading := _headed_for(squad)  # mid-move: the places moves under way are heading for
	var changed := false
	var any := true
	while any:
		any = false
		var order_by_place := PlaceOrder.front_first(squad.living(), places)
		for unit: SkirmishUnit in order_by_place:
			if heading["movers"].has(unit):
				continue  # mid-move: it takes the place its move is heading for
			var rank: int = places[0][unit.slot]
			var into_front_ok := rank > 1 or unit.preferred_position == 0
			if rank > 0 and into_front_ok and _clear_ahead(unit, places, cells, heading["cells"]):
				_step_up(unit, places, cells)
				any = true
				changed = true
				if not stepped.has(unit):
					stepped[unit] = true
					moved.append(unit)
	var remaining := squad.living()
	if not remaining.is_empty() and not remaining.any(func(u): return places[0][u.slot] == 0):
		var shift: int = remaining.reduce(
			func(least, u): return mini(least, places[0][u.slot]), 1 << 30
		)
		for unit in remaining:
			places[0][unit.slot] -= shift
		changed = true
		squad.front_distance -= squad.direction * shift * SkirmishSquad.RANK_DEPTH
	if changed:
		squad.store.columns.commit(UnitColumns.RANK, places[0])
	return moved


## {"movers": {unit: true} for the units moving in swaps under way, "cells": {Vector2i(rank,
## column): [mover, ...]} for the places their moves are heading for}.
static func _headed_for(squad: SkirmishSquad) -> Dictionary:
	var movers := {}
	var cells := {}
	for swap in squad.swaps:
		for mover in swap["to"]:
			movers[mover] = true
			var to: Vector2i = swap["to"][mover]
			for rank in range(to.x, to.x + mover.footprint_depth):
				for column in range(to.y, to.y + mover.footprint_width):
					if not cells.has(Vector2i(rank, column)):
						cells[Vector2i(rank, column)] = []
					cells[Vector2i(rank, column)].append(mover)
	return {"movers": movers, "cells": cells}


## {Vector2i(rank, column): [unit, ...]}: the places the living units' footprints cover
## (`places`: UnitPlace.table).
static func _cells(squad: SkirmishSquad, places: Array) -> Dictionary:
	var cells := {}
	for unit in squad.living():
		var at := UnitPlace.read(places, squad, unit)
		for rank in range(at.x, at.x + unit.footprint_depth):
			for column in range(at.y, at.y + unit.footprint_width):
				if not cells.has(Vector2i(rank, column)):
					cells[Vector2i(rank, column)] = []
				cells[Vector2i(rank, column)].append(unit)
	return cells


## The unit steps up a rank in `places`, and `cells` (_cells) with it.
static func _step_up(unit: SkirmishUnit, places: Array, cells: Dictionary) -> void:
	var at := Vector2i(places[0][unit.slot], places[1][unit.slot])
	for column in range(at.y, at.y + unit.footprint_width):
		cells[Vector2i(at.x + unit.footprint_depth - 1, column)].erase(unit)
		if not cells.has(Vector2i(at.x - 1, column)):
			cells[Vector2i(at.x - 1, column)] = []
		cells[Vector2i(at.x - 1, column)].append(unit)
	places[0][unit.slot] = at.x - 1


## True if no other living unit (`cells`: _cells) stands in the row ahead of the unit across
## its columns, nor is a move under way heading there (`heading`: _headed_for's cells).
static func _clear_ahead(
	unit: SkirmishUnit, places: Array, cells: Dictionary, heading: Dictionary
) -> bool:
	var at := Vector2i(places[0][unit.slot], places[1][unit.slot])
	var row := at.x - 1
	for column in range(at.y, at.y + unit.footprint_width):
		for other in cells.get(Vector2i(row, column), []):
			if other != unit:
				return false
		for mover in heading.get(Vector2i(row, column), []):
			if mover != unit:
				return false
	return true
