class_name SquadRanks
extends RefCounted
## Closing ranks after a fight (spec 27 round 6): the dead leave holes in a squad's places,
## so when its fight ends its living units are laid out again as a solid block - front
## band first, then by their old rank and column - in rows as wide as the squad, the last
## row centred. Pure over the squad it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


static func close(squad: SkirmishSquad) -> void:
	var order := squad.living()
	var places := {}  # unit -> its place, read once
	for unit in order:
		places[unit] = UnitPlace.of(squad, unit)
	order.sort_custom(
		func(a, b):
			var ka := [a.preferred_position, places[a].x, places[a].y]  # a place holds one unit
			var kb := [b.preferred_position, places[b].x, places[b].y]
			return ka < kb
	)
	var rows := _rows(order, squad.width)
	var rank := 0
	for row in rows:
		var used := 0
		var depth := 1
		for unit in row:
			used += unit.footprint_width
			depth = maxi(depth, unit.footprint_depth)
		var column := (squad.width - used) / 2
		for unit in row:
			UnitPlace.put(squad, unit, Vector2i(rank, column))
			column += unit.footprint_width
		rank += depth
	squad.swaps.clear()


## The units split into rows no wider than `width` columns.
static func _rows(order: Array, width: int) -> Array:
	var rows := [[]]
	var used := 0
	for unit in order:
		if used + unit.footprint_width > width and not rows[-1].is_empty():
			rows.append([])
			used = 0
		rows[-1].append(unit)
		used += unit.footprint_width
	return rows
