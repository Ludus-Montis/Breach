class_name FormationShuffle
extends RefCounted
## Re-forming (Decisions 46 and 49, specs/22-formation-feel-test.md): after a reinforcement
## or a death, units move toward the places their band prefers.
## - **Into free space first.** A front-preferring unit behind the front takes the nearest
##   free place in the front rank, moving sideways or diagonally if need be. In a fight, a
##   unit that joined as a reinforcement may take one beyond the squad's columns, widening
##   the line up to the combat width (Decision 51).
## - **Otherwise through.** A unit with a stronger claim (its band further forward, or the
##   same band and a higher priority) moves forward a rank past the units directly ahead in
##   its columns. Passed units that fit take its back row; any too wide or too deep move
##   back, through the formation, to the nearest free space that fits them.
## - **How long:** as far as the furthest unit travels, at the slowest involved unit's
##   speed, slowed by crowding while the squad fights (and later by terrain).
## - **No ground ceded mid-move:** until a move completes everyone keeps their place, and
##   keeps fighting. A death of anyone involved cancels it.
## A squad re-forms until no further move can start. Pure over the squad it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const SquadPlaces = preload("res://sim/skirmish/formation/squad_places.gd")
const PlaceOrder = preload("res://sim/skirmish/formation/place_order.gd")
const ShuffleFront = preload("res://sim/skirmish/formation/shuffle_front.gd")

## How terrain changes the speed units pass one another at; 1 until lanes carry terrain.
const TERRAIN_FACTOR := 1.0
## How far behind its place a passed unit may look for free space.
const SEARCH_RANKS := 64
const EPSILON := 0.000001


## True if `unit` has the stronger claim to a forward place than `other`.
static func stronger(unit: SkirmishUnit, other: SkirmishUnit) -> bool:
	if unit.preferred_position != other.preferred_position:
		return unit.preferred_position < other.preferred_position
	return unit.position_priority > other.position_priority


## One tick: active moves cancel, count down or complete; then a re-forming squad starts
## every move it can, and stops re-forming once nothing is moving. Returns "swapped" events.
static func step(
	squad: SkirmishSquad, tick_seconds: float, travel_scale: float, tick: int
) -> Array:
	var events := []
	var within := false  # every living unit known to stand in the squad's columns
	for index in range(squad.swaps.size() - 1, -1, -1):
		var swap: Dictionary = squad.swaps[index]
		if swap["to"].keys().any(func(u): return not u.is_alive()):
			squad.swaps.remove_at(index)
			continue
		swap["ticks"] -= 1
		if swap["ticks"] > 0:
			continue
		# store: scattered (a swap ending, its step-ups, a widening: per event)
		for unit in swap["to"]:
			UnitPlace.put(squad, unit, swap["to"][unit])
		squad.swaps.remove_at(index)
		if not within or not swap["to"].keys().all(func(u): return _inside(squad, u)):
			_widen_to_fit(squad)  # only the movers' columns have changed since it last did
			within = true
		var ids: Array = swap["passed"].map(func(u): return u.id)
		events.append(
			FormationEvents.unit_event("swapped", tick, squad, swap["mover"], {"passed": ids})
		)
		# store: scattered (a swap ending, its step-ups, a widening: per event)
		for moved in squad.compact():  # close the gaps the move left
			var rank := UnitPlace.rank(squad, moved)
			events.append(
				FormationEvents.unit_event("stepped_up", tick, squad, moved, {"rank": rank})
			)
	if squad.reforming and not _start_moves(squad, tick_seconds, travel_scale):
		squad.reforming = not squad.swaps.is_empty()
	return events


## True if the unit stands within the squad's columns.
static func _inside(squad: SkirmishSquad, unit: SkirmishUnit) -> bool:
	var column := UnitPlace.of(squad, unit).y
	return column >= 0 and column + unit.footprint_width <= squad.width


## {unit: the swap under way it moves in} for the squad, as it stands: give it to offset()
## when asking about many of its units.
static func movers(squad: SkirmishSquad) -> Dictionary:
	var out := {}
	for swap in squad.swaps:
		for unit in swap["to"]:
			if not out.has(unit):
				out[unit] = swap
	return out


## How far a unit is through a move: Vector2(ranks forward, columns across). `moving` is
## movers() for the squad as it stands, if the caller has it.
static func offset(squad: SkirmishSquad, unit: SkirmishUnit, moving = null) -> Vector2:
	if squad.swaps.is_empty():
		return Vector2.ZERO
	if moving != null:
		var swap = moving.get(unit)
		return Vector2.ZERO if swap == null else _progress(swap, unit)
	for swap in squad.swaps:
		if swap["to"].has(unit):
			return _progress(swap, unit)
	return Vector2.ZERO


## How far `unit` is through `swap`.
static func _progress(swap: Dictionary, unit: SkirmishUnit) -> Vector2:
	var progress := 1.0 - float(swap["ticks"]) / float(swap["total"])
	var from: Vector2i = swap["from"][unit]
	var to: Vector2i = swap["to"][unit]
	return Vector2((from.x - to.x) * progress, (to.y - from.y) * progress)


static func _start_moves(squad: SkirmishSquad, tick_seconds: float, travel_scale: float) -> bool:
	var claimed := {"cells": {}, "left": 1 << 30, "right": -(1 << 30)}  # _claim
	var occupied := SquadPlaces.occupancy(squad)  # no one moves while moves are planned
	var busy := {}  # unit -> true: those in a move
	for swap in squad.swaps:
		for unit in swap["to"]:
			busy[unit] = true
			_claim(claimed, swap["to"][unit], unit)
	var front_first := PlaceOrder.front_first(squad.living(), occupied["table"])
	var open := ShuffleFront.open(squad, front_first, claimed, occupied)
	var started := false
	for unit: SkirmishUnit in front_first:
		if occupied["places"][unit].x == 0 or busy.has(unit):
			continue
		var plan := ShuffleFront.free_place(squad, unit, claimed, [occupied, open])
		if plan.is_empty():
			plan = _through(squad, unit, busy, claimed, occupied)
		if plan.is_empty():
			continue
		var involved: Array = plan["to"].keys()
		var slowest: float = involved.reduce(func(least, u): return minf(least, u.speed), INF)
		if slowest <= 0.0:
			continue
		var crowding := (
			BattleTuning.current().scrum_crowding
			if squad.state == SkirmishSquad.State.FIGHTING
			else 1.0
		)
		var pace := travel_scale * slowest * TERRAIN_FACTOR * crowding
		var seconds: float = plan["distance"] * SkirmishSquad.RANK_DEPTH / pace
		var ticks := maxi(1, ceili(seconds / tick_seconds - EPSILON))
		plan.merge({"ticks": ticks, "total": ticks, "from": SquadPlaces.places(squad, involved)})
		squad.swaps.append(plan)
		for moving in plan["to"]:
			busy[moving] = true
			_claim(claimed, plan["to"][moving], moving)
		started = true
	return started


## Claims the cells `unit`'s footprint covers at `place` for a planned move: `claimed` is
## {"cells": SquadPlaces' claimed cells, "left", "right": the columns they span}.
static func _claim(claimed: Dictionary, place: Vector2i, unit: SkirmishUnit) -> void:
	SquadPlaces.claim(claimed["cells"], place, unit)
	if unit.footprint_depth > 0 and unit.footprint_width > 0:
		claimed["left"] = mini(claimed["left"], place.y)
		claimed["right"] = maxi(claimed["right"], place.y + unit.footprint_width)


## A front-preferring unit's move to the nearest free place in the front rank; {} if none.
## Takes in any columns units now stand in outside 0..width (a spread, Decision 51):
## renumbers every column, swaps under way included, and shifts the centre to match.
static func _widen_to_fit(squad: SkirmishSquad) -> void:
	var left := 0
	var right := squad.width
	var places := UnitPlace.table(squad)  # fetched once (spec 35)
	for unit in squad.living():
		var column: int = places[1][unit.slot]
		left = mini(left, column)
		right = maxi(right, column + unit.footprint_width)
	if left == 0 and right == squad.width:
		return
	squad.centre_shift += left + (right - left - squad.width) / 2.0
	# store: scattered (a swap ending, its step-ups, a widening: per event)
	for unit in squad.units:
		UnitPlace.put(squad, unit, UnitPlace.of(squad, unit) - Vector2i(0, left))
	for swap in squad.swaps:
		for key in ["from", "to"]:
			for unit in swap[key]:
				swap[key][unit] -= Vector2i(0, left)
	squad.width = right - left


## Moving forward a rank past weaker units ahead; {} if it can't (Decision 49).
static func _through(
	squad: SkirmishSquad,
	unit: SkirmishUnit,
	busy: Dictionary,
	claimed: Dictionary,
	occupied: Dictionary
) -> Dictionary:
	var ahead := SquadPlaces.ahead(occupied, unit)
	if ahead.is_empty() or ahead.any(func(a): return busy.has(a) or not stronger(unit, a)):
		return {}
	var places: Dictionary = occupied["places"]
	var at: Vector2i = places[unit]
	var to := {unit: at - Vector2i(1, 0)}
	var back_row := at.x + unit.footprint_depth - 1
	var moving := ahead + [unit]
	var taken := {}  # the cells this move claims, over those claimed already
	SquadPlaces.claim(taken, to[unit], unit)
	var distance := 1
	for passed in ahead:
		var was: Vector2i = places[passed]
		var place := Vector2i(back_row, was.y)
		if not _fits_back_row(unit, at, passed, was):
			place = _nearest_free_behind(squad, passed, moving, [taken, occupied, claimed["cells"]])
			if place.x < 0:
				return {}
			distance = maxi(distance, maxi(place.x - was.x, absi(place.y - was.y)))
		to[passed] = place
		SquadPlaces.claim(taken, place, passed)
	return {"mover": unit, "passed": ahead, "to": to, "distance": distance}


## The nearest place behind `unit` where its whole footprint is free; (-1, -1) if none.
## `held`: [the cells this move claims, SquadPlaces.occupancy, the cells claimed before it].
static func _nearest_free_behind(
	squad: SkirmishSquad, unit: SkirmishUnit, moving: Array, held: Array
) -> Vector2i:
	var best := Vector2i(-1, -1)
	var best_key := Vector3i(1 << 30, 0, 0)
	var at: Vector2i = held[1]["places"][unit]
	for rank in range(at.x + 1, at.x + SEARCH_RANKS):
		for column in range(squad.width - unit.footprint_width + 1):
			var place := Vector2i(rank, column)
			var lateral := absi(column - at.y)
			var key := Vector3i(maxi(rank - at.x, lateral), lateral, rank)
			if key < best_key and _vacant(place, unit, moving, held):
				best = place
				best_key = key
	return best


## SquadPlaces.vacant over both layers of claims (`held`: as _nearest_free_behind's).
static func _vacant(place: Vector2i, unit: SkirmishUnit, moving: Array, held: Array) -> bool:
	if not SquadPlaces.vacant(held[1], place, unit, moving, held[0]):
		return false
	for rank in range(place.x, place.x + unit.footprint_depth):
		for column in range(place.y, place.y + unit.footprint_width):
			if held[2].has(Vector2i(rank, column)):
				return false
	return true


## A passed unit (at `was`) fits the row the mover (at `at`) leaves: one rank deep,
## directly ahead, within its columns.
static func _fits_back_row(
	unit: SkirmishUnit, at: Vector2i, passed: SkirmishUnit, was: Vector2i
) -> bool:
	return (
		passed.footprint_depth == 1
		and was.x == at.x - 1
		and was.y >= at.y
		and was.y + passed.footprint_width <= at.y + unit.footprint_width
	)
