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
## A squad re-forms until no further move can start. Pure over the squad it is given. Which
## moves start is the crate's (native/rust/src/march/shuffle.rs and front.rs, spec 39 step
## 3d), and so are the swaps' ends, the widening and the closing up after (swaps.rs, step
## 8a2-iii); the list of swaps under way and the events stay here.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")


## One tick: active moves cancel, count down or complete; then a re-forming squad starts
## every move it can, and stops re-forming once nothing is moving. Returns "swapped" events.
static func step(
	squad: SkirmishSquad, tick_seconds: float, travel_scale: float, tick: int
) -> Array:
	var events := [] if squad.swaps.is_empty() else _end_swaps(squad, tick)
	if squad.reforming and not _start_moves(squad, tick_seconds, travel_scale):
		squad.reforming = not squad.swaps.is_empty()
	return events


## The crate cancels, counts down and ends the squad's swaps under way, widens it and closes
## it up (swaps.rs); this keeps the list, renumbered, and makes the events.
static func _end_swaps(squad: SkirmishSquad, tick: int) -> Array:
	var living := squad.living()
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	var front := squad.front_distance
	var closers := SquadCompact.closers(living)
	var slots := PackedInt32Array(squad.units.map(func(unit): return unit.slot))
	var swaps := _under_way(squad)
	var ends: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.squad_swaps_end(squad.slot, closers, slots, swaps)
	)
	SquadCompact.placed(squad, front)
	var was: Array = squad.swaps
	var left := Vector2i(0, ends[0])
	var kept := []
	var at := 2
	for _each in ends[1]:
		var swap: Dictionary = was[ends[at]]
		swap["ticks"] = ends[at + 1]
		for key in ["from", "to"]:
			for unit in swap[key]:
				swap[key][unit] -= left
		kept.append(swap)
		at += 2
	squad.swaps = kept
	var events := []
	while at < ends.size():
		var swap: Dictionary = was[ends[at]]
		var ids: Array = swap["passed"].map(func(u): return u.id)
		events.append(
			FormationEvents.unit_event("swapped", tick, squad, swap["mover"], {"passed": ids})
		)
		for _each in ends[at + 1]:
			at += 2
			var rank: int = ends[at + 1]
			var moved: SkirmishUnit = living[ends[at]]
			events.append(
				FormationEvents.unit_event("stepped_up", tick, squad, moved, {"rank": rank})
			)
		at += 2
	return events


## The squad's swaps under way as the crate takes them: each its ticks left, its units'
## count, then six numbers a unit, the mover first: store slot, 1 if it stands, the place
## (rank, column) it heads for, footprint width and depth.
static func _under_way(squad: SkirmishSquad) -> PackedInt64Array:
	var out := PackedInt64Array()
	for swap in squad.swaps:
		out.append_array([swap["ticks"], swap["to"].size()])
		for unit in swap["to"]:
			var to: Vector2i = swap["to"][unit]
			var shape := [unit.footprint_width, unit.footprint_depth]
			out.append_array([unit.slot, int(unit.is_alive()), to.x, to.y] + shape)
	return out


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
	var living := squad.living()
	var index_of := {}  # unit -> where it comes in the squad's living units
	for index in living.size():
		index_of[living[index]] = index
	var swaps := PackedInt32Array()  # where each unit of a move under way goes
	var busy := PackedInt32Array()
	for swap in squad.swaps:
		for unit in swap["to"]:
			var to: Vector2i = swap["to"][unit]
			swaps.append_array(
				PackedInt32Array([to.x, to.y, unit.footprint_width, unit.footprint_depth])
			)
			if index_of.has(unit):
				busy.append(index_of[unit])
	var table := UnitView.read(squad, UnitView.PLACES)  # no one moves while moves are planned
	var ranks: PackedInt32Array = table.rank
	var files: PackedInt32Array = table.column
	var paces: PackedFloat64Array = UnitView.read(squad, [UnitColumns.SPEED]).speed
	var units := PackedInt32Array()
	var speeds := PackedFloat64Array()
	for unit in living:
		var shape := [unit.footprint_width, unit.footprint_depth]
		var claim := [unit.preferred_position, unit.position_priority, int(squad.joined.has(unit))]
		units.append_array(PackedInt32Array([ranks[unit.slot], files[unit.slot]] + shape + claim))
		speeds.append(paces[unit.slot])
	var fighting := squad.state == SkirmishSquad.State.FIGHTING
	var shape := PackedFloat64Array(
		[squad.width, squad.combat_width, squad.centre_shift, float(fighting)]
	)
	var pace := PackedFloat64Array(
		[tick_seconds, travel_scale, BattleTuning.current().scrum_crowding]
	)
	var moves: PackedInt64Array = CrateMarch.core().start_moves(
		shape, units, speeds, swaps, busy, pace
	)
	var at := 0
	while at < moves.size():
		var ticks: int = moves[at]
		var plan := {"distance": moves[at + 1], "to": {}, "from": {}}
		var count: int = moves[at + 2]
		at += 3
		for _each in count:
			var unit: SkirmishUnit = living[moves[at]]
			plan["to"][unit] = Vector2i(moves[at + 1], moves[at + 2])
			plan["from"][unit] = Vector2i(ranks[unit.slot], files[unit.slot])
			at += 3
		plan["mover"] = plan["to"].keys()[0]
		var passed := []
		for _each in moves[at]:
			at += 1
			passed.append(living[moves[at]])
		at += 1
		plan["passed"] = passed
		plan.merge({"ticks": ticks, "total": ticks})
		squad.swaps.append(plan)
	return not moves.is_empty()
