class_name GroundBodies
extends RefCounted
## Bodies on the ground (Decision 121, spec 28 part 6): the downed and the dead lie where
## they fell, no longer bodies that push (UnitBodies) but obstacles a marching formation's
## front must step over - slowed by the weight of what it crosses against its own (a grem
## over a grem by half, bodies_ground_drag), and blocked by one at least
## bodies_ground_block times its mass. Every unit walking to its place, or loose in a
## fight, is slowed the same way (underfoot). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const LYING := [SkirmishUnit.State.DOWNED, SkirmishUnit.State.DEAD]


## The share (0 to 1) of its pace the squad keeps this tick for the bodies its front rank
## is stepping over; `bodies` are the bodies lying on the field (ground()), or {"squads":
## the squads} to find them the first time it is asked, and keep them there.
static func drag(squad: SkirmishSquad, bodies: Dictionary) -> float:
	if not bodies.has("lying"):
		bodies.merge(ground(bodies["squads"]))
	if bodies["lying"].is_empty():
		return 1.0
	var ahead := UnitMotion.vector(squad.heading) * 0.5
	var points: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	var worst := 1.0
	for walker in squad.fighters():
		worst = minf(worst, underfoot(walker, points[walker.slot] + ahead, bodies))
	return worst


## The bodies lying on the field (lying_in), found by where they lie: taken once a tick,
## for every walker's step (underfoot).
static func ground(squads: Array) -> Dictionary:
	var lying := lying_in(squads)
	return of_lying(lying, null if lying.is_empty() else squads[0].store.columns)


## A grid over `lying` bodies (BodyGrid.of_bodies) where they lie (`columns`, the store's),
## with "lying": the bodies, and "ats": where each lies.
static func of_lying(lying: Array, columns: UnitColumns) -> Dictionary:
	var ats := []
	if not lying.is_empty():
		var points: PackedVector2Array = columns.column(UnitColumns.POSITION)
		ats = lying.map(func(body): return points[body.slot])
	var bodies := []
	for index in range(lying.size()):
		bodies.append([ats[index], ScrumReach.radius(lying[index]), lying[index]])
	var grid := BodyGrid.of_bodies(bodies)
	grid["lying"] = lying
	grid["ats"] = ats
	return grid


## The bodies lying on the field: the downed and the dead.
static func lying_in(squads: Array) -> Array:
	var lying := []
	for other in squads:
		for unit in other.units:
			if LYING.has(unit.state):
				lying.append(unit)
	return lying


## The share (0 to 1) of its pace `walker` keeps stepping onto `step` over the bodies on
## the ground, `bodies` (of_lying; spec 30 round 3: every unit that walks, in its place or loose):
## slowed by the weight of what it crosses against its own, blocked by one
## bodies_ground_block times its mass. Only the bodies that could touch the step are
## looked at; the least share is the same whichever order they come in.
static func underfoot(walker: SkirmishUnit, step: Vector2, bodies: Dictionary) -> float:
	var tuning := BattleTuning.current()
	var own := float(walker.footprint_width * walker.footprint_depth)
	var lying: Array = bodies["lying"]
	var ats: Array = bodies["ats"]
	var near: float = ScrumReach.radius(walker) + bodies["widest"] + BodyGrid.MARGIN
	var worst := 1.0
	for index in BodyGrid.near(bodies, step, near):
		var body: SkirmishUnit = lying[index]
		var reach := ScrumReach.radius(walker) + ScrumReach.radius(body)
		if body == walker or ats[index].distance_to(step) >= reach:
			continue
		var weight: float = body.footprint_width * body.footprint_depth / own
		if weight >= tuning.bodies_ground_block:
			return 0.0
		worst = minf(worst, 1.0 / (1.0 + weight * tuning.bodies_ground_drag))
	return worst
