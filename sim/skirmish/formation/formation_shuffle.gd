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
## 8a2-iii), and the swaps under way themselves (step 8b4-v); the events stay here.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")


## One tick: active moves cancel, count down or complete; then a re-forming squad starts
## every move it can, and stops re-forming once nothing is moving. Returns "swapped" events.
static func step(
	squad: SkirmishSquad, tick_seconds: float, travel_scale: float, tick: int
) -> Array:
	var events := _end_swaps(squad, tick) if swapping(squad) else []
	if squad.reforming and not _start_moves(squad, tick_seconds, travel_scale):
		squad.reforming = swapping(squad)
	return events


## The crate cancels, counts down and ends the squad's swaps under way, widens it and closes
## it up (swaps.rs); this makes the events.
static func _end_swaps(squad: SkirmishSquad, tick: int) -> Array:
	var living := squad.living()
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	var front := squad.front_distance
	var closers := SquadCompact.closers(living)
	var slots := PackedInt32Array(squad.units.map(func(unit): return unit.slot))
	var ends: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.squad_swaps_end(squad.slot, closers, slots)
	)
	SquadCompact.placed(squad, front)
	var events := []
	var at := 0
	while at < ends.size():
		var mover: int = ends[at]
		var ids := []
		for passed in ends.slice(at + 2, at + 2 + ends[at + 1]):
			ids.append(passed)
		at += 2 + ends[at + 1]
		var extra := {"unit": mover, "passed": ids}
		events.append(FormationEvents.squad_event("swapped", tick, squad, extra))
		for _each in ends[at]:
			var moved: SkirmishUnit = living[ends[at + 1]]
			var rank: int = ends[at + 2]
			events.append(
				FormationEvents.unit_event("stepped_up", tick, squad, moved, {"rank": rank})
			)
			at += 2
		at += 1
	return events


## Whether any swap of the squad's re-forming is under way.
static func swapping(squad: SkirmishSquad) -> bool:
	return squad.store.field().squad_swapping(squad.slot)


## The squad's swaps under way are cancelled.
static func clear(squad: SkirmishSquad) -> void:
	squad.store.field().squad_clear_swaps(squad.slot)


## {unit id: how far it is through the swap under way it moves in} for the squad, as it
## stands: give it to offset() when asking about many of its units.
static func movers(squad: SkirmishSquad) -> Dictionary:
	var out := {}
	var flat: PackedFloat64Array = squad.store.field().squad_swap_offsets(squad.slot)
	for at in range(0, flat.size(), 3):
		out[int(flat[at])] = Vector2(flat[at + 1], flat[at + 2])
	return out


## How far a unit is through a move: Vector2(ranks forward, columns across). `moving` is
## movers() for the squad as it stands, if the caller has it.
static func offset(squad: SkirmishSquad, unit: SkirmishUnit, moving = null) -> Vector2:
	if moving == null:
		if not swapping(squad):
			return Vector2.ZERO
		moving = movers(squad)
	return moving.get(unit.id, Vector2.ZERO)


static func _start_moves(squad: SkirmishSquad, tick_seconds: float, travel_scale: float) -> bool:
	var living := squad.living()
	var listed := PackedInt64Array()  # [id, store slot] each living unit
	for unit in living:
		listed.append_array([unit.id, unit.slot])
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
	return squad.store.field().squad_start_moves(squad.slot, listed, shape, units, speeds, pace)
