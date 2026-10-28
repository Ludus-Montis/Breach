class_name RoutePatch
extends RefCounted
## A learned detour (spec 30): a way round that worked, kept as a patch to a route -
## "between `from_along` and `to_along` cells along it, follow these waypoints". The
## command a group follows keeps its patches beside its route and hands every group
## `RoutePatch.patched(route, patches)` to follow, so every group of the command takes the
## detour. Patches apply in order along the route; one starting inside an earlier one's
## stretch gives way to it. Ties go to where they lie, never to the order they were learned
## in (Decision 97). A view over the crate's (native/rust/src/route.rs, spec 39 step 3b).

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

## Where along the route the detour leaves it and rejoins it (cells).
var from_along: float
var to_along: float
## The detour's waypoints between leaving and rejoining.
var waypoints: PackedVector2Array


## The patch a detour makes: from where its first waypoint lies along `route` to where its
## last does, through the waypoints between.
static func of_detour(route: FormationRoute, detour: PackedVector2Array) -> RoutePatch:
	var along: PackedFloat64Array = CrateMoves.core().of_detour(route.points(), detour)
	return RoutePatch.new(along[0], along[1], detour.slice(1, -1))


## `route` with each patch's stretch replaced by its detour.
static func patched(route: FormationRoute, patches: Array) -> FormationRoute:
	var flat := RoutePatch.flat(patches)
	var points: PackedVector2Array = CrateMoves.core().patched(
		route.points(), flat[0], flat[1], flat[2], flat[3]
	)
	return FormationRoute.new(points, route.corridor_half_width)


## `patches` as the crate takes them: [where each leaves, where each rejoins, how many
## waypoints each has, all their waypoints in order].
static func flat(patches: Array) -> Array:
	var froms := PackedFloat64Array()
	var tos := PackedFloat64Array()
	var counts := PackedInt32Array()
	var waypoints := PackedVector2Array()
	for patch: RoutePatch in patches:
		froms.append(patch.from_along)
		tos.append(patch.to_along)
		counts.append(patch.waypoints.size())
		waypoints.append_array(patch.waypoints)
	return [froms, tos, counts, waypoints]


func _init(leaves: float, rejoins: float, detour: PackedVector2Array) -> void:
	from_along = leaves
	to_along = rejoins
	waypoints = detour
