class_name UnitSteer
extends RefCounted
## Steering round bodies (Decision 114, spec 30 round 2): a unit walking to a goal whose
## straight way there runs into a body that won't part for it - a foe's, or another squad's
## (its own squad's part: UnitBodies) - within a few cells (bodies_steer_look, BattleTuning)
## steps round the nearest such body, on the side the body leans least into its way - the
## side nearer its goal - aiming to graze it just clear; past it, it walks straight on. A
## body dead centre in its way is passed on the side a seeded draw picks (Decision 97: no
## handedness, never the list). A body standing on the goal itself is left to the bodies
## parting (UnitBodies). One rule for seekers, regrouping units and stragglers. A view over
## the crate's (native/rust/src/steer.rs, spec 39 step 3b); the bodies near the way are
## found here (BodyGrid) until the scrum ports. Cells.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")


## The point the unit at `at` heads for this step on its way to `goal`: the goal, or a
## point beside the body in its way. `bodies` = [[point, radius, unit], ...] (ScrumSlots);
## `crowd`, a BodyGrid.of_bodies over them if given, keeps the look to those near.
static func toward(
	unit: SkirmishUnit,
	at: Vector2,
	goal: Vector2,
	bodies: Array,
	fight_seed: int,
	crowd: Dictionary = {}
) -> Vector2:
	var tuning := BattleTuning.current()
	var look := tuning.bodies_steer_look
	var own := ScrumReach.radius(unit)
	var points := PackedVector2Array()
	var radii := PackedFloat64Array()
	var squads := PackedInt64Array()
	var ids := PackedInt64Array()
	var way := goal - at
	var length := way.length()
	if length >= 0.000001:
		for body in _near(bodies, crowd, at, at + way / length * minf(length, look), own):
			points.append(body[0])
			radii.append(body[1])
			squads.append(body[2].squad_id)
			ids.append(body[2].id)
	var steering := PackedFloat64Array([look, tuning.bodies_steer_clear])
	return CrateMoves.core().steer(
		at, goal, own, unit.squad_id, unit.id, steering, fight_seed, points, radii, squads, ids
	)


## The bodies that could stand within a body's breadth (`own` and theirs) of the way from
## `at` to `end`, in the list's order: all of them without a crowd.
static func _near(bodies: Array, crowd: Dictionary, at: Vector2, end: Vector2, own: float) -> Array:
	if crowd.is_empty():
		return bodies
	var out := []
	for found in BodyGrid.near(crowd, at, own + crowd["widest"] + 0.01, end):
		out.append(bodies[found])
	return out
