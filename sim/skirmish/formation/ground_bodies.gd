class_name GroundBodies
extends RefCounted
## Bodies on the ground (Decision 121, spec 28 part 6): the downed and the dead lie where
## they fell, no longer bodies that push (UnitBodies) but obstacles a marching formation's
## front must step over - slowed by the weight of what it crosses against its own (a grem
## over a grem by half, bodies_ground_drag), and blocked by one at least
## bodies_ground_block times its mass. Every unit walking to its place, or loose in a
## fight, is slowed the same way. How much is the crate's (`lying.rs`; the front's share,
## `drag`, since spec 39 step 8a2-ii); which bodies lie, and where, are found here. Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const LYING := [SkirmishUnit.State.DOWNED, SkirmishUnit.State.DEAD]


## The share (0 to 1) of its pace the squad keeps this tick for the bodies its front rank
## is stepping over; `bodies` are the bodies lying on the field (ground()), or {"squads":
## the squads} to find them the first time it is asked, and keep them there. `roster`: the
## tick's (CrateRoster) of those squads, made of this squad alone if none is given.
static func drag(squad: SkirmishSquad, bodies: Dictionary, roster: Object = null) -> float:
	if not bodies.has("lying"):
		bodies.merge(ground(bodies["squads"]))
	if bodies["lying"].is_empty():
		return 1.0
	var listed := 0
	if roster == null:
		roster = CrateRoster.of([squad])
	else:
		listed = bodies["squads"].find(squad)
	var lying := CrateMarch.lying(bodies)
	var tuning := BattleTuning.current()
	var weights := PackedFloat64Array([tuning.bodies_ground_drag, tuning.bodies_ground_block])
	return squad.store.field().ground_drag(roster, listed, lying[0], lying[1], weights)


## The bodies lying on the field (lying_in), found by where they lie: taken once a tick,
## for every walker's step.
static func ground(squads: Array) -> Dictionary:
	var lying := lying_in(squads)
	return of_lying(lying, null if lying.is_empty() else squads[0].store.columns)


## {"lying": the `lying` bodies, "ats": where each lies (`columns`, the store's)}.
static func of_lying(lying: Array, columns: UnitColumns) -> Dictionary:
	var ats := []
	if not lying.is_empty():
		var points: PackedVector2Array = columns.column(UnitColumns.POSITION)
		ats = lying.map(func(body): return points[body.slot])
	return {"lying": lying, "ats": ats}


## The bodies lying on the field: the downed and the dead.
static func lying_in(squads: Array) -> Array:
	var lying := []
	for other in squads:
		for unit in other.units:
			if LYING.has(unit.state):
				lying.append(unit)
	return lying
