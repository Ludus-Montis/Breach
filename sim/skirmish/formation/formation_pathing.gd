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
## (path_relook cells). Pure over the squads it is given. The look, the group's walker and
## its pathfinder are the crate's (native/rust/src/pathing.rs, spec 39 step 3c), and so are
## which groups follow their command's patches and which look (detours.rs, step 8b3), and
## the commands' patches themselves (commands.rs, step 8b4-iii).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")


## One tick: groups follow their commands' patched routes, then marching groups look
## ahead and keep any way round they find.
static func step(
	squads: Array, terrain: FormationTerrain, tick: int, fight_seed: int, events: Array
) -> void:
	if terrain == null:
		return
	for squad in squads:
		_follow(squad)
	var relook := BattleTuning.current().path_relook
	for squad in squads:
		if squad.store.field().path_looks(squad.slot, squad.route != null, relook):
			_look_ahead(squad, terrain, tick, fight_seed, events)


## The group as one walker: its tallest height, swimming or climbing only if every unit
## can, at the least of their abilities, stamina and speed.
static func group_walker(squad: SkirmishSquad) -> TerrainWalker:
	var members := CratePaths.members(squad, squad.living())
	var fields: PackedFloat64Array = CratePaths.core().group_walker(members)
	var walker := TerrainWalker.new(
		fields[0], int(fields[1]), int(fields[2]), fields[3] != 0.0, fields[4] != 0.0
	)
	walker.stamina = fields[5]
	walker.speed = fields[6]
	return walker


## Re-routes the group onto its command's route with the command's patches, if they have
## changed since it last followed it; it stands where it stood along it. The crate's.
static func _follow(squad: SkirmishSquad) -> void:
	var field: Object = squad.store.field()
	if not field.path_follows(squad.slot):
		return
	var points: PackedVector2Array = field.path_follow(squad.slot)
	squad.route = FormationRoute.new(points, squad.command.route.corridor_half_width)


## Looks along the route ahead; keeps a way round on the command where one is better.
static func _look_ahead(
	squad: SkirmishSquad, terrain: FormationTerrain, tick: int, fight_seed: int, events: Array
) -> void:
	var tuning := BattleTuning.current()
	var front := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
	var looks := PackedFloat64Array(
		[tuning.path_look, tuning.path_detour_gain, tuning.sight_budget]
	)
	looks.append_array(
		PackedFloat64Array(
			[tuning.ground_cliff_quarters, tuning.stamina_climb, tuning.stamina_swim]
		)
	)
	var way: PackedVector2Array = CratePaths.core().look_ahead(
		terrain.core,
		squad.route.points(),
		front,
		squad.direction,
		CratePaths.members(squad, squad.living()),
		fight_seed,
		looks
	)
	if way.is_empty():
		return
	squad.command.learn(way)
	events.append(FormationEvents.squad_event("detoured", tick, squad, {"cells": way.size()}))
