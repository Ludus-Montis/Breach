class_name FormationRendezvous
extends RefCounted
## Planned rendezvous (Decision 87, spec 27 round 2): the overlord's timing, given before
## departure, so waves on different routes reach their points together. The march is
## predicted as FormationSimulation runs it - a cell pace per tick, slowed by the ground
## along the route (Decision 85), sweeping round bends (Decision 105) as fast as its
## files, walking to their places, keep within their slack (FormationWalk, spec 30 round
## 3) - so it holds until something interferes on the way (a fight, a queue, a gap it
## pours through). Pure. The prediction is the crate's (`rendezvous.rs`, spec 39 step 5b).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")


## Ticks for a squad `width` wide moving `cells_per_second` to reach `cells` along `route`,
## setting out from `from` cells along it, over `terrain` if given, its units walking to
## their places within `slack` cells (FormationWalk; negative: the slack at discipline
## 50).
static func ticks_to(
	route: FormationRoute,
	cells: float,
	width: int,
	cells_per_second: float,
	tick_seconds: float,
	from: float = 0.0,
	terrain: FormationTerrain = null,
	slack: float = -1.0
) -> int:
	var march := PackedFloat64Array(
		[
			cells,
			width,
			cells_per_second,
			tick_seconds,
			from,
			slack if slack >= 0.0 else _middling_slack()
		]
	)
	return CrateMarch.core().rendezvous_ticks(route.points(), march, UnitMover.ground(terrain))


static func _middling_slack() -> float:
	var tuning := BattleTuning.current()
	return lerpf(tuning.walk_slack_loose, tuning.walk_slack_drilled, 0.5)


## Ticks each wave should wait before setting out so all arrive together: {key: ticks}
## from {key: predicted ticks}.
static func waits(predicted: Dictionary) -> Dictionary:
	var waited: PackedInt64Array = CrateMarch.core().rendezvous_waits(
		PackedInt64Array(predicted.values())
	)
	var out := {}
	for index in predicted.size():
		out[predicted.keys()[index]] = waited[index]
	return out
