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
## 8a2-iii), the swaps under way themselves (step 8b4-v), and the whole tick of it with its
## events, one pass over every squad (swaps/pass.rs, step 8b5-xxxii); this takes the events.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## One tick over `squads`, in the lists' order: each one's moves under way cancel, count down
## or complete; then a re-forming squad starts every move it can, and stops re-forming once
## nothing is moving. Returns the "swapped" and "stepped_up" events.
static func step(squads: Array, tick_seconds: float, travel_scale: float, tick: int) -> Array:
	if squads.is_empty():
		return []
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var pace := PackedFloat64Array(
		[tick_seconds, travel_scale, BattleTuning.current().scrum_crowding]
	)
	ColumnsLent.all(columns, func(): return field.shuffle_step(listed, pace, tick))
	return CrateEvents.taken(field)  # "swapped", "stepped_up"


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
