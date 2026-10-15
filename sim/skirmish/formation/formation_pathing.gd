class_name FormationPathing
extends RefCounted
## Groups find their way (spec 30 round 3, part 6b). A marching group looks along its route
## ahead (path_look cells) as its pathfinder sees it - the unit with the best wits
## (SquadPathfinder), its sight shaped and obscured (PathSight) - for the whole group as
## one walker: as tall as its tallest, swimming or climbing only if all of them can, at the
## least of their abilities. Where that stretch is barred, or much slower than a way round
## (path_detour_gain), it plans the way round (TerrainPaths.plan) and keeps it on its
## command as a patch to the route (RoutePatch): every group of the command follows the
## patched route from then on. It looks again only once its front has moved on
## (path_relook cells). Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const TerrainPaths = preload("res://sim/skirmish/formation/terrain_paths.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const RoutePatch = preload("res://sim/skirmish/formation/route_patch.gd")
const SquadPathfinder = preload("res://sim/skirmish/formation/squad_pathfinder.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## One tick: groups follow their commands' patched routes, then marching groups look
## ahead and keep any way round they find.
static func step(
	squads: Array, terrain: FormationTerrain, tick: int, fight_seed: int, events: Array
) -> void:
	if terrain == null:
		return
	for squad in squads:
		_follow(squad)
	for squad in squads:
		if squad.state == SkirmishSquad.State.MOVING and squad.route != null:
			if _looks(squad):
				_look_ahead(squad, terrain, tick, fight_seed, events)


## The group as one walker: its tallest height, swimming or climbing only if every unit
## can, at the least of their abilities, stamina and speed.
static func group_walker(squad: SkirmishSquad) -> TerrainWalker:
	var living := squad.living()
	var walkers := living.map(func(u): return TerrainWalker.of_unit(u))
	if walkers.is_empty():
		return TerrainWalker.grounded(1.0)
	var walker := TerrainWalker.new(
		walkers.map(func(w): return w.height).max(),
		walkers.map(func(w): return w.swimmer).min(),
		walkers.map(func(w): return w.climber).min(),
		walkers.all(func(w): return w.swims),
		walkers.all(func(w): return w.climbs)
	)
	walker.stamina = walkers.map(func(w): return w.stamina).min()
	walker.speed = walkers.map(func(w): return w.speed).min()
	return walker


## Re-routes the group onto its command's route with the command's patches, if they have
## changed since it last followed it.
static func _follow(squad: SkirmishSquad) -> void:
	var command = squad.command
	if command.route == null or squad.patched_count == command.patches.size():
		return
	squad.patched_count = command.patches.size()
	var at := squad.position
	var far_end := squad.home_distance > 0.0
	squad.route = RoutePatch.patched(command.route, command.patches)
	var cells := squad.route.length_cells()
	squad.front_distance = squad.route.distance_of(at) / MapLayoutDef.CELLS_PER_TILE
	if far_end:
		squad.home_distance = cells / MapLayoutDef.CELLS_PER_TILE


static func _looks(squad: SkirmishSquad) -> bool:
	var cells := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	return absf(cells - squad.looked_at) >= BattleTuning.current().path_relook


## Looks along the route ahead; keeps a way round on the command where one is better.
static func _look_ahead(
	squad: SkirmishSquad, terrain: FormationTerrain, tick: int, fight_seed: int, events: Array
) -> void:
	var tuning := BattleTuning.current()
	var front := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	squad.looked_at = front
	var length := squad.route.length_cells()
	var ahead := clampf(front + squad.direction * tuning.path_look, 0.0, length)
	var from := squad.route.point_at(front)
	var to := squad.route.point_at(ahead)
	if from.distance_to(to) < 1.0:
		return
	var walker := group_walker(squad)
	var straight := TerrainPaths.cost(terrain, walker, PackedVector2Array([from, to]))
	if straight <= from.distance_to(to) * (1.0 + tuning.path_detour_gain):
		return  # open enough: it keeps to its route
	var finder := SquadPathfinder.pick(squad, fight_seed)
	var sight: PathSight = null
	if finder != null and finder.definition != null:
		var at: Vector2 = squad.store.columns.value(finder.slot, UnitColumns.POSITION)
		sight = PathSight.of(finder.definition, at, to - from, terrain)
	var plan := TerrainPaths.plan(terrain, walker, from, to, sight)
	var way: PackedVector2Array = plan["waypoints"]
	if not plan["reaches"] or way.size() < 3:
		return
	var round_cost := TerrainPaths.cost(terrain, walker, way)
	if round_cost >= straight * (1.0 - tuning.path_detour_gain):
		return  # no better than going straight on
	if squad.direction < 0:
		way.reverse()
	squad.command.patches.append(RoutePatch.of_detour(squad.command.route, way))
	events.append(FormationEvents.squad_event("detoured", tick, squad, {"cells": way.size()}))
