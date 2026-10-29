class_name FormationMarch
extends RefCounted
## A squad's march along its route (Decisions 74 and 75, spec 27 round 1): how long its
## route is, which way it faces there, reaching either end, and where its units stand for
## views - along the route (`distance`, for the one-lane scene) and in cells (`position`).
## Pure over the squads it is given; FormationSimulation calls it each tick. Where its front
## gets, which way it goes, its pace and its facing, and its units' walk, are the crate's
## (native/rust/src/march/, spec 39 step 3d), and so are its ends (step 8a2-v), each march
## and its events (the move's pass, step 8b5-viii); the units' placing and the routers' and
## the loose stay here until the tick (step 8b) ports.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationPour = preload("res://sim/skirmish/formation/formation_pour.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
## The columns the walk to places sets and steps, held for each squad's pass (UnitView):
## where each stands and faces, its pace, the stamina a climb or a swim spends, how far along
## the route it stands, and its climb (the crate's walk modes: where it last stood, whether
## it is on a face, the climb left; spec 35 step 7c).
const WALKED := [
	UnitColumns.POSITION,
	UnitColumns.BEARING,
	UnitColumns.SPEED,
	UnitColumns.STAMINA,
	UnitColumns.DISTANCE,
	UnitColumns.FOOTHOLD,
	UnitColumns.ON_FACE,
	UnitColumns.CLIMB_LEFT,
]

## How a march ends (`march_end`'s first number): it marches on, it arrived, it returned.
const ENDED_NOT := 0.0
const ENDED_ARRIVED := 1.0

## What sync_units holds: WALKED, and the places, loose points and routes it reads.
const SYNCED := WALKED + UnitView.PLACES + [UnitColumns.LOOSE_AT] + FormationRout.ROUTE


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
## loose and fleeing units stand where their own moves took them.
static func sync_units(
	squads: Array, timing := [], terrain: FormationTerrain = null, fight_seed: int = 0
) -> void:
	if not timing.is_empty():
		timing = timing + [GroundBodies.ground(squads)]
	# Points and bearings held for the pass (Decision 146), once for every squad (Decision 150):
	# set in place here, and the units in their places walk after through the hold, each
	# reading only its own place, the ground and the lying; released (committed) once. Their
	# places, loose points and routes, which nothing here writes, are held with them, so each
	# squad's reads of them (FormationPour.targets' too) index the hold's.
	var all_held := UnitView.hold(squads, SYNCED)
	for entry in squads:
		var moving := FormationShuffle.movers(entry)
		var held := UnitView.of(entry)  # once per squad: an index, not a copy
		var places := held
		var ranks: PackedInt32Array = held.rank
		var aims := {}  # where each unit in its place makes for, worked out once a squad
		if not timing.is_empty():
			aims = FormationPour.targets(entry, terrain, moving, fight_seed)
		var points: PackedVector2Array = held.position
		var facing: PackedFloat64Array = held.bearing
		var distances: PackedFloat64Array = held.distance
		var footholds: PackedVector2Array = held.foothold
		var loose_at: PackedVector2Array = held.at
		var route: UnitView = null if entry.fleeing.is_empty() else held
		var walkers := []
		# read once a squad (Decision 144): nothing here moves its frame
		var basis: Array = entry.frame()
		var front: float = entry.front_distance
		var direction: int = entry.direction
		var heading: float = entry.heading
		for unit in entry.units:
			if not unit.is_alive():
				continue  # the fallen lie where they fell (Decision 126)
			var swapping := FormationShuffle.offset(entry, unit, moving)
			distances[unit.slot] = (
				(front - direction * ranks[unit.slot] * SkirmishSquad.RANK_DEPTH)  # rank_distance
				+ direction * swapping.x * SkirmishSquad.RANK_DEPTH
			)
			if entry.fleeing.has(unit.id):
				# placement: a router's point on its route, moved by RoutFlight (Decision 139)
				points[unit.slot] = FormationRout.where_in(route, entry, unit)
				if not entry.loose.has(unit.id):
					facing[unit.slot] = heading
			elif entry.loose.has(unit.id):
				# placement: its loose point, moved by UnitMover (Decision 139)
				points[unit.slot] = loose_at[unit.slot]
			elif timing.is_empty():
				footholds[unit.slot] = FormationWalk.place_of(entry, basis, unit, moving, places)
				# placement: its frame's slot, set before any walking (Decision 139)
				points[unit.slot] = footholds[unit.slot]
				facing[unit.slot] = heading
			else:
				walkers.append(unit)
		FormationWalk.walk_all(entry, held, walkers, aims, timing, terrain)
	UnitView.release(all_held)  # the routers', the loose, the placed and the walked points


## Where `squad` stands along its route: [front, home, order, direction].
static func _along(squad) -> PackedFloat64Array:
	return PackedFloat64Array(
		[squad.front_distance, squad.home_distance, squad.order, squad.direction]
	)
