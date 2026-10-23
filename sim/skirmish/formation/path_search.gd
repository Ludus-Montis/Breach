class_name PathSearch
extends RefCounted
## One A* search over the terrain grid for a walker (TerrainPaths, spec 30): the quickest
## way, not the shortest, eight ways from each cell, no corner cut, keeping to the grid,
## its ties going to geometry. Given a sight (PathSight) it plans on what it sees or has
## seen (PathMemory, which it adds to) - unseen cells count as open ground - and keeps the
## stretch in sight: the whole way if the goal is in sight and reached (`reaches`), else
## up to the edge of sight. The goal is a cell, or a route: rejoining it at the first route
## point reached that lies ahead of where the group left it, else the first behind. A
## walker with a stamina plans only climbs it lasts. A view over the crate's search
## (native/rust/src/paths/search.rs, spec 39 step 3c), which holds the rules. Pure.

const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const PathMemory = preload("res://sim/skirmish/formation/path_memory.gd")

## Cells taken off the frontier and expanded, for measuring.
var expanded := 0
## Whether the way found reaches the goal (false: it ends on the edge of sight).
var reaches := false

var _terrain: FormationTerrain
var _walker: TerrainWalker
var _start: Vector2i
var _sight: PathSight
var _memory: PathMemory


## The cells of the quickest way from start to `goal`, or to the edge of sight where the
## best plan runs out of it; empty if there is none.
func to_cell(goal: Vector2i) -> Array[Vector2i]:
	return _run(goal, PackedVector2Array(), 0.0, false)


## The cells of the quickest way from start onto `route` at or ahead of `progress` cells
## along it - with none in reach, behind it - or to the edge of sight where that way
## leaves it; empty if there is none.
func to_route(route: FormationRoute, progress: float) -> Array[Vector2i]:
	return _run(Vector2i(-1, -1), route.points(), progress, true)


func _run(
	goal: Vector2i, route: PackedVector2Array, progress: float, routed: bool
) -> Array[Vector2i]:
	var out: Dictionary = CratePaths.core().search(
		_terrain.core,
		CratePaths.walker(_walker),
		_start,
		goal,
		route,
		progress,
		routed,
		CratePaths.sight(_sight),
		CratePaths.known(_memory)
	)
	CratePaths.learnt(_memory, out)
	expanded += out["expanded"]
	reaches = out["reaches"]
	return out["cells"]


func _init(
	terrain: FormationTerrain,
	walker: TerrainWalker,
	start: Vector2i,
	sight: PathSight = null,
	memory: PathMemory = null
) -> void:
	_terrain = terrain
	_walker = walker
	_start = start
	_sight = sight
	_memory = memory
