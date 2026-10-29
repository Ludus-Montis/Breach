class_name FormationMarch
extends RefCounted
## A squad's march along its route (Decisions 74 and 75, spec 27 round 1): how long its
## route is, which way it faces there, reaching either end, and where its units stand for
## views - along the route (`distance`, for the one-lane scene) and in cells (`position`).
## Pure over the squads it is given; FormationSimulation calls it each tick. Where its front
## gets, which way it goes, its pace and its facing, and its units' walk, are the crate's
## (native/rust/src/march/, spec 39 step 3d), and so are its ends (step 8a2-v), each march
## and its events (the move's pass, step 8b5-viii), and the units' placing and walking
## (`march::sync`, step 8b5-xxxiii).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## How a march ends (`march_end`'s first number): it marches on, it arrived, it returned.
const ENDED_NOT := 0.0
const ENDED_ARRIVED := 1.0


## Turns the squad's frame at once to its route's heading at its front: where it is placed
## (on the march it sweeps round bends: FormationSweep).
static func face(squad: SkirmishSquad) -> void:
	squad.store.field().march_face(squad.slot)  # along the route the crate holds


## Arrival at the enemy's end (the fort is immune in the feel test), or a retreat home: its
## front there, or every unit of it home (FormationWithdraw's test, within HOME_CELLS)
## wherever its front stands - it holds as it faces, rather than about-face to march a
## rank's length back to home and about-face again. Which, if either, is the crate's
## (`march::end`, spec 39 step 8a2-v); a retreat hands it its roster for the units' test.
static func check_ends(mover: SkirmishSquad, route_end: float, tick: int, events: Array) -> void:
	var roster: Object = null
	if mover.order == SkirmishUnit.Order.RETREAT:
		roster = CrateRoster.of([mover])
	var ended: PackedFloat64Array = mover.store.field().march_end(
		_along(mover), route_end, mover.slot, roster, 0
	)
	if ended[0] == ENDED_NOT:
		return
	mover.front_distance = ended[1]
	if ended[0] == ENDED_ARRIVED:
		mover.state = SkirmishSquad.State.ARRIVED
		events.append(FormationEvents.squad_event("arrived", tick, mover))
	else:
		mover.order = SkirmishUnit.Order.HOLD
		mover.state = SkirmishSquad.State.HOLDING
		events.append(FormationEvents.squad_event("returned", tick, mover))


## Units follow their squad's placement - including any swap under way - and views read
## DISTANCE (along the route). Placing (no `timing`) sets each unit on its place;
## otherwise a unit in its place walks to where it makes for ([seconds, cells a second at
## speed 1], FormationWalk; FormationPour.targets, its queues' draws by `fight_seed`), and
## loose and fleeing units stand where their own moves took them. One crate pass over every
## squad (`march::sync`, spec 39 step 8b5-xxxiii); a fall from a climb face lands its hurt
## there, onto the fell queue.
static func sync_units(
	squads: Array, timing := [], terrain: FormationTerrain = null, fight_seed: int = 0
) -> void:
	if squads.is_empty():
		return
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var ground: Object = null if timing.is_empty() else UnitMover.ground(terrain)
	var tuning := CrateMarch.walk_tuning()
	tuning.append(BattleTuning.current().wounds_reach)
	var at := PackedFloat64Array(timing)
	ColumnsLent.all(columns, func(): field.march_sync(listed, at, ground, fight_seed, tuning))


## Where `squad` stands along its route: [front, home, order, direction].
static func _along(squad) -> PackedFloat64Array:
	return PackedFloat64Array(
		[squad.front_distance, squad.home_distance, squad.order, squad.direction]
	)
