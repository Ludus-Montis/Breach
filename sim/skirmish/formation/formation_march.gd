class_name FormationMarch
extends RefCounted
## A squad's march along its route (Decisions 74 and 75, spec 27 round 1): how long its
## route is, which way it faces there, reaching either end, and where its units stand for
## views - along the route (`distance`, for the one-lane scene) and in cells (`position`).
## Pure over the squads it is given; FormationSimulation calls it each tick.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationPour = preload("res://sim/skirmish/formation/formation_pour.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## The squad's route length in tiles (`fallback` for a squad without a route).
static func length(squad: SkirmishSquad, fallback: float) -> float:
	if squad.route == null:
		return fallback
	return squad.route.length_cells() / MapLayoutDef.CELLS_PER_TILE


## How far the squad moves this tick on `terrain` (Decision 85): `step` at the pace of the
## worst cell its front rank steps into that it can cross - a unit whose way is barred
## squeezes in and pours through (FormationWalk.target_of, spec 30 round 3). Where none of
## its front can go it halts, reported once as "blocked".
static func pace(
	squad: SkirmishSquad, terrain: FormationTerrain, step: float, tick: int, events: Array
) -> float:
	if terrain == null:
		return step
	var ahead := UnitMotion.vector(squad.heading)
	var points: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	var worst := INF
	for unit in squad.fighters():
		var at := points[unit.slot]
		var factor := terrain.factor(unit, at, at + ahead)
		if factor > 0.0:  # one stepping onto ground it can't cross pours (FormationWalk)
			worst = minf(worst, factor)
	worst = 0.0 if worst == INF else worst
	if worst <= 0.0 and not squad.blocked:
		events.append(FormationEvents.squad_event("blocked", tick, squad))
	squad.blocked = worst <= 0.0
	return step * worst


## Where the squad's front gets moving `delta` tiles along its route of `route_end` tiles:
## never past either end, nor past the point its order takes it to.
static func toward(squad: SkirmishSquad, delta: float, route_end: float) -> float:
	var next := clampf(squad.front_distance + delta, 0.0, route_end)
	var enemy_end := route_end if is_zero_approx(squad.home_distance) else 0.0
	var target := squad.home_distance if squad.order == SkirmishUnit.Order.RETREAT else enemy_end
	var before := signf(target - squad.front_distance)
	if before != 0.0 and signf(target - next) != before:
		return target  # it would pass its target: it stops there
	return next


## Which way the squad's order takes it along its route: +1 up it, -1 down it. Advancing
## heads for the end away from home, retreating for home.
static func travel_sign(squad: SkirmishSquad, route_end: float) -> int:
	var enemy_end := route_end if is_zero_approx(squad.home_distance) else 0.0
	var advancing := squad.order == SkirmishUnit.Order.ADVANCE
	var target := enemy_end if advancing else squad.home_distance
	if is_equal_approx(target, squad.front_distance):
		return squad.direction
	return 1 if target > squad.front_distance else -1


## Turns the squad's frame at once to its route's heading at its front: where it is placed
## (on the march it sweeps round bends: FormationSweep).
static func face(squad: SkirmishSquad) -> void:
	if squad.route == null:
		return
	var cells := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	var way := squad.route.heading_at(cells) * squad.direction
	squad.heading = UnitMotion.bearing_to(Vector2.ZERO, way, squad.heading)


## Arrival at the enemy's end (the fort is immune in the feel test), or a retreat home: its
## front there, or every unit of it home (FormationWithdraw's test, within HOME_CELLS)
## wherever its front stands - it holds as it faces, rather than about-face to march a
## rank's length back to home and about-face again.
static func check_ends(mover: SkirmishSquad, route_end: float, tick: int, events: Array) -> void:
	var enemy_end := route_end if is_zero_approx(mover.home_distance) else 0.0
	if (
		mover.order == SkirmishUnit.Order.ADVANCE
		and is_equal_approx(mover.front_distance, enemy_end)
	):
		mover.front_distance = enemy_end
		mover.state = SkirmishSquad.State.ARRIVED
		events.append(FormationEvents.squad_event("arrived", tick, mover))
	elif (
		mover.order == SkirmishUnit.Order.RETREAT
		and (
			is_equal_approx(mover.front_distance, mover.home_distance)
			or (mover.route != null and FormationWithdraw._home(mover))
		)
	):
		mover.front_distance = mover.home_distance
		mover.order = SkirmishUnit.Order.HOLD
		mover.state = SkirmishSquad.State.HOLDING
		events.append(FormationEvents.squad_event("returned", tick, mover))


## Units follow their squad's placement - including any swap under way - and views read
## unit.distance (along the route). Placing (no `timing`) sets each unit on its place;
## otherwise a unit in its place walks to where it makes for ([seconds, cells a second at
## speed 1], FormationWalk; FormationPour.targets, its queues' draws by `fight_seed`), and
## loose and fleeing units stand where their own moves took them.
static func sync_units(
	squads: Array, timing := [], terrain: FormationTerrain = null, fight_seed: int = 0
) -> void:
	if not timing.is_empty():
		timing = timing + [GroundBodies.ground(squads)]
	for entry in squads:
		var moving := FormationShuffle.movers(entry)
		var columns: UnitColumns = entry.store.columns
		var ranks: PackedInt32Array = columns.column(UnitColumns.RANK)  # fetched once
		var aims := {}  # where each unit in its place makes for, worked out once a squad
		if not timing.is_empty():
			aims = FormationPour.targets(entry, terrain, moving, fight_seed)
		# Each column fetched once and committed once (Decision 144); the units in their
		# places walk after, each reading only its own place, the ground and the lying.
		var points: PackedVector2Array = columns.column(UnitColumns.POSITION)
		var facing: PackedFloat64Array = columns.column(UnitColumns.BEARING)
		var loose_at: PackedVector2Array = columns.column(UnitColumns.LOOSE_AT)
		var places := UnitPlace.table(entry)
		var walkers := []
		for unit in entry.units:
			if not unit.is_alive():
				continue  # the fallen lie where they fell (Decision 126)
			var swapping := FormationShuffle.offset(entry, unit, moving)
			unit.distance = (
				entry.rank_distance(ranks[unit.slot])
				+ entry.direction * swapping.x * SkirmishSquad.RANK_DEPTH
			)
			if entry.fleeing.has(unit.id):
				# placement: a router's point on its route, moved by RoutFlight (Decision 139)
				points[unit.slot] = FormationRout.where(entry, unit.id)
				if not entry.loose.has(unit.id):
					facing[unit.slot] = entry.heading
			elif entry.loose.has(unit.id):
				# placement: its loose point, moved by UnitMover (Decision 139)
				points[unit.slot] = loose_at[unit.slot]
			elif timing.is_empty():
				unit.foothold = FormationWalk.place_of(entry, unit, moving, places)
				# placement: its frame's slot, set before any walking (Decision 139)
				points[unit.slot] = unit.foothold
				facing[unit.slot] = entry.heading
			else:
				walkers.append(unit)
		# placement: the routers', the loose and the placed points above (Decision 139)
		columns.commit(UnitColumns.POSITION, points)
		columns.commit(UnitColumns.BEARING, facing)
		for unit in walkers:
			FormationWalk.walk(entry, unit, aims[unit.id], timing, terrain)
