class_name FormationSweep
extends RefCounted
## A formation sweeps round its route's bends (Decision 105, spec 30 round 1): as it
## marches, its frame turns towards its route's heading where its front is, no faster than
## its outer file can march the arc - pivoting on its front centre, the file half its width
## out covers a quarter circle in the time a wheel took (SquadTurn). A 5 degree bend and a
## 90 degree bend follow the same rule, so it doesn't halt to turn; at a sharp bend it cuts
## the corner a little. Going back the way it faces is an about-face instead (ScrumTurn).
## Pure over the squad it is given. A view over the crate's (native/rust/src/march/sweep.rs,
## spec 39 step 3d).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")


## Degrees a second the squad's frame may turn marching at `cells_per_second`.
static func rate(squad: SkirmishSquad, cells_per_second: float) -> float:
	return CrateMarch.core().sweep_rate(squad.width, cells_per_second)


## Turns the squad's frame towards its route's heading at its front, by what its rate allows
## in `seconds`.
static func step(squad: SkirmishSquad, cells_per_second: float, seconds: float) -> void:
	if squad.route == null:
		return
	var cells := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	squad.heading = CrateMarch.core().sweep(
		squad.route.points(),
		cells,
		squad.direction,
		squad.heading,
		squad.width,
		cells_per_second,
		seconds
	)
