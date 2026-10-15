class_name SquadPlaces
extends RefCounted
## Who stands where in a squad's frame, by rank and column (spec 30 round 3): the places
## its living units' footprints cover, taken once while none moves, so a re-form
## (FormationShuffle) asks a place who is on it rather than asking every unit. The answers
## are those of looking through every living unit, in the same order. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## {"cells": {Vector2i(rank, column): [unit, ...]}, "order": {unit: where it comes in the
## squad's living units}, "places": {unit: Vector2i(rank, column)}} as the squad's living
## units stand now.
static func occupancy(squad: SkirmishSquad) -> Dictionary:
	var cells := {}
	var order := {}
	var places := {}
	var living := squad.living()
	for index in range(living.size()):
		var unit: SkirmishUnit = living[index]
		order[unit] = index
		var at := UnitPlace.of(squad, unit)
		places[unit] = at
		for rank in range(at.x, at.x + unit.footprint_depth):
			for column in range(at.y, at.y + unit.footprint_width):
				var cell := Vector2i(rank, column)
				if not cells.has(cell):
					cells[cell] = []
				cells[cell].append(unit)
	return {"cells": cells, "order": order, "places": places}


## True if `unit`'s footprint at `place` is free of everyone but the units `moving`, and of
## every cell a planned move claims.
static func vacant(
	occupied: Dictionary, place: Vector2i, unit: SkirmishUnit, moving: Array, claimed: Dictionary
) -> bool:
	var cells: Dictionary = occupied["cells"]
	for rank in range(place.x, place.x + unit.footprint_depth):
		for column in range(place.y, place.y + unit.footprint_width):
			var cell := Vector2i(rank, column)
			if claimed.has(cell):
				return false
			for other in cells.get(cell, []):
				if not moving.has(other):
					return false
	return true


## The living units in the row directly ahead of `unit`, across its columns, in the order
## they come in the squad's living units.
static func ahead(occupied: Dictionary, unit: SkirmishUnit) -> Array:
	var cells: Dictionary = occupied["cells"]
	var order: Dictionary = occupied["order"]
	var at: Vector2i = occupied["places"][unit]
	var out := []
	for column in range(at.y, at.y + unit.footprint_width):
		for other in cells.get(Vector2i(at.x - 1, column), []):
			if other != unit and not out.has(other):
				out.append(other)
	out.sort_custom(func(a, b): return order[a] < order[b])
	return out


## Marks the cells `unit`'s footprint covers at `place` as claimed.
static func claim(claimed: Dictionary, place: Vector2i, unit: SkirmishUnit) -> void:
	for rank in range(place.x, place.x + unit.footprint_depth):
		for column in range(place.y, place.y + unit.footprint_width):
			claimed[Vector2i(rank, column)] = true


## {unit: Vector2i(rank, column)} for the units of `squad`.
static func places(squad: SkirmishSquad, units: Array) -> Dictionary:
	var out := {}
	for unit in units:
		out[unit] = UnitPlace.of(squad, unit)
	return out
