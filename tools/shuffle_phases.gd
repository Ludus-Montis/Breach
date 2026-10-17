extends RefCounted
## FormationShuffle.step, part by part, for tick_phases: the moves under way (counting
## down, completing, compacting), the planning of new moves (who stands where, front-first
## order, the free front places), and each unit's tries - a free front place, then passing
## through. It mirrors step(), so tick_phases' own check catches it drifting. `lap` is
## tick_phases' timer: lap.call(part) books the time since the last lap to `part`.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const PlaceOrder = preload("res://sim/skirmish/formation/place_order.gd")
const ShuffleFront = preload("res://sim/skirmish/formation/shuffle_front.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SquadPlaces = preload("res://sim/skirmish/formation/squad_places.gd")


static func step(squad: SkirmishSquad, paced: Array, tick: int, lap: Callable) -> Array:
	var events := []
	var within := false
	for index in range(squad.swaps.size() - 1, -1, -1):
		var swap: Dictionary = squad.swaps[index]
		if swap["to"].keys().any(func(u): return not u.is_alive()):
			squad.swaps.remove_at(index)
			continue
		swap["ticks"] -= 1
		if swap["ticks"] > 0:
			continue
		for unit in swap["to"]:
			UnitPlace.put(squad, unit, swap["to"][unit])
		squad.swaps.remove_at(index)
		if (
			not within
			or not swap["to"].keys().all(func(u): return FormationShuffle._inside(squad, u))
		):
			FormationShuffle._widen_to_fit(squad)
			within = true
		var ids: Array = swap["passed"].map(func(u): return u.id)
		events.append(
			FormationEvents.unit_event("swapped", tick, squad, swap["mover"], {"passed": ids})
		)
		for moved in squad.compact():
			var rank := UnitPlace.rank(squad, moved)
			events.append(
				FormationEvents.unit_event("stepped_up", tick, squad, moved, {"rank": rank})
			)
	lap.call("shuffle: moves under way")
	if squad.reforming and not _start_moves(squad, paced, lap):
		squad.reforming = not squad.swaps.is_empty()
	return events


## FormationShuffle._start_moves, timed. `paced`: [tick_seconds, travel_scale].
static func _start_moves(squad: SkirmishSquad, paced: Array, lap: Callable) -> bool:
	var claimed := {"cells": {}, "left": 1 << 30, "right": -(1 << 30)}
	var occupied := SquadPlaces.occupancy(squad)
	var busy := {}
	for swap in squad.swaps:
		for unit in swap["to"]:
			busy[unit] = true
			FormationShuffle._claim(claimed, swap["to"][unit], unit)
	lap.call("shuffle: plan: who stands where")
	var table: UnitView = occupied["table"]
	var front_first := PlaceOrder.front_first(squad.living(), table.rank, table.column)
	lap.call("shuffle: plan: front-first order")
	var open := ShuffleFront.open(squad, front_first, claimed, occupied)
	lap.call("shuffle: plan: free front places")
	var started := false
	for unit in front_first:
		if occupied["places"][unit].x == 0 or busy.has(unit):
			continue
		var plan := ShuffleFront.free_place(squad, unit, claimed, [occupied, open])
		lap.call("shuffle: try: free front place")
		if plan.is_empty():
			plan = FormationShuffle._through(squad, unit, busy, claimed, occupied)
			lap.call("shuffle: try: pass through")
		if plan.is_empty():
			continue
		started = _begin(squad, plan, paced, [busy, claimed]) or started
		lap.call("shuffle: begin a move")
	lap.call("shuffle: begin a move")
	return started


## Starts `plan` as _start_moves does, if all involved can move. `held`: [busy, claimed].
static func _begin(squad: SkirmishSquad, plan: Dictionary, paced: Array, held: Array) -> bool:
	var involved: Array = plan["to"].keys()
	var paces: PackedFloat64Array = squad.store.columns.column(UnitColumns.SPEED)
	var slowest: float = involved.reduce(func(least, u): return minf(least, paces[u.slot]), INF)
	if slowest <= 0.0:
		return false
	var crowding := (
		BattleTuning.current().scrum_crowding
		if squad.state == SkirmishSquad.State.FIGHTING
		else 1.0
	)
	var pace: float = paced[1] * slowest * FormationShuffle.TERRAIN_FACTOR * crowding
	var seconds: float = plan["distance"] * SkirmishSquad.RANK_DEPTH / pace
	var ticks := maxi(1, ceili(seconds / paced[0] - FormationShuffle.EPSILON))
	plan.merge({"ticks": ticks, "total": ticks, "from": SquadPlaces.places(squad, involved)})
	squad.swaps.append(plan)
	for moving in plan["to"]:
		held[0][moving] = true
		FormationShuffle._claim(held[1], plan["to"][moving], moving)
	return true
