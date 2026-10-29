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
## (path_relook cells). Pure over the squads it is given. The pass is the crate's
## (native/rust/src/pathing/pass.rs, spec 39 step 8b5-vii): who follows and who looks
## (detours.rs), the look (pathing.rs), what it reads of each unit (its walker, wits, place
## and definition's sight, all held by the crate) and the "detoured" event, built as data;
## this takes the events (CrateEvents). A group that re-routed reads its route from the
## crate (SkirmishSquad.route, spec 39 step 8b5-xxxvii).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")


## One tick: groups follow their commands' patched routes, then marching groups look
## ahead and keep any way round they find. Adds "detoured" events.
static func step(
	squads: Array, terrain: FormationTerrain, tick: int, fight_seed: int, events: Array
) -> void:
	if terrain == null or squads.is_empty():
		return
	var slots := PackedInt32Array()
	for squad in squads:
		slots.append(squad.slot)
	var field: Object = squads[0].store.field()
	field.path_step(terrain.core, slots, tick, fight_seed, looks())  # the re-routed: the crate's
	events.append_array(CrateEvents.taken(field))


## The tuning a look reads: [path_relook, path_look, path_detour_gain, sight_budget,
## ground_cliff_quarters, stamina_climb, stamina_swim].
static func looks() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var out := PackedFloat64Array(
		[tuning.path_relook, tuning.path_look, tuning.path_detour_gain, tuning.sight_budget]
	)
	out.append_array(
		PackedFloat64Array(
			[tuning.ground_cliff_quarters, tuning.stamina_climb, tuning.stamina_swim]
		)
	)
	return out


## The group as one walker: its tallest height, swimming or climbing only if every unit
## can, at the least of their abilities, stamina and speed.
static func group_walker(squad: SkirmishSquad) -> TerrainWalker:
	var fields: PackedFloat64Array = CratePaths.core().group_walker(CratePaths.members(squad))
	var walker := TerrainWalker.new(
		fields[0], int(fields[1]), int(fields[2]), fields[3] != 0.0, fields[4] != 0.0
	)
	walker.stamina = fields[5]
	walker.speed = fields[6]
	return walker
