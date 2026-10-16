class_name FormationPursuit
extends RefCounted
## A formation pursuing as a whole (Decision 95, spec 27 round 8): when its enemy retreats
## it stays locked on it and its frame advances after it as a body, along the route its
## quarry flees by (Decision 113: a route is a way to travel, not the formation's own) - no
## faster than its rearmost unit keeps up (Decision 103) - its units seeking contact as
## they go at the march pace. When the enemy is out of its sight, gone, or no longer
## retreating, or it has gone as far from its post as its discipline leashes it
## (FormationDiscipline.pursuit_leash, Decision 107), or most of it has tired
## (FormationStamina), it gives up: it marches back along the
## route it is on to the post it held, takes up its own route there and turns to face the
## way it held it, taking up its order again. Squads keep `pursuit` ({"foe", "route",
## "post", "post_at", "home", "direction", "order", "returning"}). Pure over the squads.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const FormationSweep = preload("res://sim/skirmish/formation/formation_sweep.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

## It advances while its enemy is more than this many cells ahead of its front, and its
## foremost unit no more than reach_pursuit_lag (BattleTuning) behind it.
const CLOSE_ENOUGH := 1.0
## How near (cells) a route must pass a pursuer for it to join it.
const JOIN_CELLS := 2.0


## Sets `squad` pursuing the retreating `foe`, along the route its quarry flees by (Decision
## 113): it joins that route where it stands, and follows it towards the foe.
static func begin(squad: SkirmishSquad, foe: SkirmishSquad, tick: int, events: Array) -> void:
	FormationLocks.lock(squad, foe)
	if squad.pursuit.is_empty():
		squad.pursuit = {
			"foe": foe.id,
			"route": squad.route,
			"post": squad.front_distance,
			"post_at": squad.position,
			"home": squad.home_distance,
			"direction": squad.direction,
			"order": squad.order,
			"returning": false,
		}
		_take(squad, foe.route, foe.position)
	events.append(FormationEvents.squad_event("pursuing", tick, squad, {"of": foe.id}))


## The squad takes `route` from where it stands, travelling towards `towards`; it keeps its
## own if that route doesn't pass by it.
static func _take(squad: SkirmishSquad, route, towards: Vector2) -> void:
	if route == null or squad.route == null or route == squad.route:
		return
	var along: float = route.distance_of(squad.position)
	if route.point_at(along).distance_to(squad.position) > JOIN_CELLS:
		return
	squad.route = route
	squad.front_distance = along / MapLayoutDef.CELLS_PER_TILE
	squad.direction = 1 if route.distance_of(towards) >= along else -1


## One tick of pursuit: pursuers advance after their enemy or give up and go back.
static func step(
	squads: Array, tick: int, cells_per_second: float, seconds: float, events: Array
) -> void:
	var by_id := {}
	for entry in squads:
		by_id[entry.id] = entry
	for squad in squads:
		if squad.pursuit.is_empty() or squad.is_destroyed():
			continue
		if squad.pursuit["returning"]:
			_arrive(squad, tick, events)
			continue
		var foe: SkirmishSquad = by_id.get(squad.pursuit["foe"])
		if _given_up(squad, foe):
			_go_back(squad, squads, tick, events)
		else:
			_advance(squad, foe, cells_per_second, seconds)


## [cells its frame has gone from its post, as the crow flies, cells its leash allows (INF:
## none)] for a pursuing squad.
static func reach(squad: SkirmishSquad) -> Array:
	var gone: float = squad.position.distance_to(squad.pursuit["post_at"])
	return [gone, FormationDiscipline.pursuit_leash(squad)]


static func _given_up(squad: SkirmishSquad, foe: SkirmishSquad) -> bool:
	if foe == null or foe.is_destroyed() or foe.order != SkirmishUnit.Order.RETREAT:
		return true
	if not FormationSight.detects(squad, foe) or FormationStamina.squad_gives_up(squad):
		return true  # out of sight, or too tired to go on (spec 28 part 7)
	var reached := reach(squad)
	return reached[0] >= reached[1]


## Its frame moves along its route towards the enemy while the enemy is ahead.
static func _advance(
	squad: SkirmishSquad, foe: SkirmishSquad, cells_per_second: float, seconds: float
) -> void:
	var ahead := _ahead(squad, _spread(foe).get_center())
	if ahead <= CLOSE_ENOUGH or _lagging(squad):
		return
	FormationSweep.step(squad, squad.speed() * cells_per_second, seconds)  # round bends
	var tiles := minf(squad.speed() * cells_per_second * seconds, ahead - CLOSE_ENOUGH)
	tiles /= MapLayoutDef.CELLS_PER_TILE
	var length := 1e9
	if squad.route != null:
		length = squad.route.length_cells() / MapLayoutDef.CELLS_PER_TILE
	squad.front_distance = clampf(squad.front_distance + squad.direction * tiles, 0.0, length)


## Cells `at` lies ahead of the squad's front along the route it travels (as the crow flies
## along its heading, with no route).
static func _ahead(squad: SkirmishSquad, at: Vector2) -> float:
	if squad.route == null:
		return (at - squad.position).dot(UnitMotion.vector(squad.heading))
	var own := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	return (squad.route.distance_of(at) - own) * squad.direction


## True if any of its units lags more than reach_pursuit_lag behind its place: a formation
## pursues as a body, its frame waiting for its rearmost (Decision 103).
static func _lagging(squad: SkirmishSquad) -> bool:
	var forward := UnitMotion.vector(squad.heading)
	var places := UnitView.read(squad, UnitView.PLACES)  # read once (Decision 146)
	for unit in squad.living():
		if squad.chasers.has(unit.id):
			continue  # one out on its own makes its own way (Decision 112)
		var anchor := ScrumStance.anchor(squad, unit, places)
		var behind := (anchor - ScrumReach.at(squad, unit)).dot(forward)
		if behind > BattleTuning.current().reach_pursuit_lag:
			return true
	return false


## It ends its fight and marches back to its post (a march home, with home its post).
static func _go_back(squad: SkirmishSquad, squads: Array, tick: int, events: Array) -> void:
	FormationLocks.release(squad, squads)
	squad.pursuit["returning"] = true
	var post_at: Vector2 = squad.pursuit["post_at"]  # back along the route it is on
	squad.home_distance = squad.route.distance_of(post_at) / MapLayoutDef.CELLS_PER_TILE
	squad.order = SkirmishUnit.Order.RETREAT
	events.append(FormationEvents.squad_event("pursuit_ended", tick, squad))


## Back at its post (its march home ended), it faces the way it held it and takes up its
## order again.
static func _arrive(squad: SkirmishSquad, tick: int, events: Array) -> void:
	if squad.order == SkirmishUnit.Order.RETREAT:
		return  # still marching back
	var held: Dictionary = squad.pursuit
	squad.route = held["route"]  # at its post: its own route again
	squad.front_distance = held["post"]
	squad.home_distance = held["home"]
	squad.order = held["order"]
	squad.pursuit = {}
	ScrumTurn.begin(squad, held["direction"], tick, events)


## The cells a squad's units cover where they actually stand - in its frame, loose in a
## fight or fleeing a withdrawal - not where its frame is.
static func _spread(squad: SkirmishSquad) -> Rect2:
	var living := squad.living()
	if living.is_empty():
		return SquadEdges.bounds(squad)
	var out := ScrumReach.area(squad, living[0])
	for unit in living:
		out = out.merge(ScrumReach.area(squad, unit))
	return out
