class_name TerrainPaths
extends RefCounted
## Paths over the terrain grid for a unit (spec 30; Decisions 75, 85 and 97): the quickest
## way at the walker's own pace on each cell - the pace the march uses (the crate's shares,
## from FormationTerrain.crossing): ground, slope, liquid bands by its height, swimming and
## climbing by its abilities. A ford beats swimming, or a wood a detour, by cost alone.
## There is no leash: a pathfinder plans on what it sees (PathSight) - seen cells at their
## real cost, unseen ones as open ground - and where the best plan runs out of sight it
## heads for that edge and plans again from there (PathSearch), remembering what it has
## seen (PathMemory, optional) so a wall felt along stays known. Deterministic: ties go to
## geometry, so the same ground gives the same path and the ground mirrored gives it
## mirrored. A view over the crate's paths (native/rust/src/paths/, spec 39 step 3c), which
## hold the rules. Pure.
##
##   TerrainPaths.plan(terrain, walker, from, goal, sight)    -> {waypoints, reaches, cells}
##   TerrainPaths.rejoin(terrain, walker, from, route, sight) -> {..., along}
##   TerrainPaths.cells(terrain, walker, from, goal, sight)   -> the cells of the way, or []
##   TerrainPaths.cost(terrain, walker, waypoints)            -> seconds at speed 1

const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const PathSearch = preload("res://sim/skirmish/formation/path_search.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const PathMemory = preload("res://sim/skirmish/formation/path_memory.gd")


## The cells of the quickest way for `walker` from the cell holding `from` to the cell
## holding `goal`, both included, planned on what `sight` sees (null: everything) and
## `memory` has seen - to the edge of sight where the best plan runs out of it; empty
## where there is no way.
static func cells(
	terrain: FormationTerrain,
	walker: TerrainWalker,
	from: Vector2,
	goal: Vector2,
	sight: PathSight = null,
	memory: PathMemory = null
) -> Array[Vector2i]:
	return PathSearch.new(terrain, walker, _cell(from), sight, memory).to_cell(_cell(goal))


## The way planned from `from` towards `goal` on what `sight` sees: {"waypoints": from,
## the centres of the cells where it turns, and `goal` - or the cell on the edge of sight
## it heads for; "reaches": whether it reaches the goal; "cells": the way's cells}. The
## waypoints are empty where there is no way.
static func plan(
	terrain: FormationTerrain,
	walker: TerrainWalker,
	from: Vector2,
	goal: Vector2,
	sight: PathSight = null,
	memory: PathMemory = null
) -> Dictionary:
	var out: Dictionary = CratePaths.core().plan(
		terrain.core,
		CratePaths.walker(walker),
		from,
		goal,
		CratePaths.sight(sight),
		CratePaths.known(memory)
	)
	return _kept(out, memory)


## The way back onto `route` from `from` on what `sight` sees: to the first cell on the
## route reached that lies at or ahead of `progress` cells along it, where the group left
## it (negative: where `from` lies along it) - with none in reach, the first behind - its
## distance along the route "along"; or where that way leaves sight, to the edge of sight
## ("reaches" false, "along" -1). As plan() otherwise.
static func rejoin(
	terrain: FormationTerrain,
	walker: TerrainWalker,
	from: Vector2,
	route: FormationRoute,
	sight: PathSight = null,
	memory: PathMemory = null,
	progress: float = -1.0
) -> Dictionary:
	var out: Dictionary = CratePaths.core().rejoin(
		terrain.core,
		CratePaths.walker(walker),
		from,
		route.points(),
		progress,
		CratePaths.sight(sight),
		CratePaths.known(memory)
	)
	return _kept(out, memory)


## The time (seconds, at a speed of a cell a second) `walker` takes along `waypoints`,
## sampled every fifth of a cell; INF if a leg crosses where it can't go.
static func cost(
	terrain: FormationTerrain, walker: TerrainWalker, waypoints: PackedVector2Array
) -> float:
	return CratePaths.core().cost(terrain.core, walker.fields(), waypoints)


## The crate's plan `out`, what it saw kept in `memory`.
static func _kept(out: Dictionary, memory: PathMemory) -> Dictionary:
	CratePaths.learnt(memory, out)
	out.erase("known")
	return out


static func _cell(point: Vector2) -> Vector2i:
	return Vector2i(floori(point.x), floori(point.y))
