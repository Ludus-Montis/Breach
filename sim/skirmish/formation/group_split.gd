class_name GroupSplit
extends RefCounted
## A group's units fallen behind go on as a group of their own (spec 30 round 3, part 4):
## a cluster of the group's units (each within group_join of another) cut off from the
## rest, all further from their places than walk_lost - the frame no longer waits for them
## (FormationWalk), unless no man is left behind - splits off as a group. Each new group
## follows the same command and orders, along the same route, its front where its foremost
## unit stands;
## its units line up across it by where they stand. New groups are numbered in the order
## of their units' seeded draws, never by ids or lists (Decision 97). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## Where units stand and their places, read once a pass (Decision 150): nobody moves until
## every split is found.
const STANDING := [UnitColumns.POSITION, UnitColumns.RANK, UnitColumns.COLUMN]
const AT_EASE := [
	SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING, SkirmishSquad.State.ARRIVED
]


## Splits off every group's fallen-behind units; new groups are numbered from `next_id`
## and added to `squads`. Returns the next free squad id.
static func step(squads: Array, tick: int, fight_seed: int, events: Array, next_id: int) -> int:
	var splits := []  # [draw, squad, cluster, where its store's units stand]
	var views := {}  # store -> STANDING, read once a pass
	for squad in squads:
		if squad.state not in AT_EASE or squad.route == null:
			continue
		var standing := UnitView.read_once(views, squad, STANDING)
		var at: PackedVector2Array = standing.position
		for cluster in _clusters(_behind(squad, standing), at):
			var least: int = cluster.map(func(u): return ScrumContest.draw(u, fight_seed)).min()
			splits.append([least, squad, cluster, at])
	splits.sort_custom(func(a, b): return a[0] < b[0])
	for split in splits:  # no split moves a unit: where they stand holds for every one
		var group := _group(split[1], split[2], fight_seed, split[3])
		group.id = next_id
		for unit in group.units:
			unit.squad_id = next_id
		next_id += 1
		squads.append(group)
		events.append(FormationEvents.squad_event("split", tick, group, {"from": split[1].id}))
	return next_id


## The squad's units in their places, fallen behind them and cut off from the rest: no
## unit of the group within its cluster stands near its place. None if the frame waits
## for every unit, or if every cluster is behind (the frame is where they are going).
## `standing` holds STANDING, as read for the pass.
static func _behind(squad: SkirmishSquad, standing: UnitView) -> Array:
	if FormationWalk.rule_of(squad) == "no_man_left_behind":
		return []
	var lost := BattleTuning.current().walk_lost
	var placed := squad.living().filter(
		func(u):
			return (
				not squad.loose.has(u.id)
				and not squad.fleeing.has(u.id)
				and not squad.taking.has(u.id)
			)
	)
	var far := {}  # unit id -> whether it stands further than walk_lost from its place
	var at: PackedVector2Array = standing.position
	for unit in placed:
		var place := FormationWalk.place_of(squad, unit, null, standing)
		far[unit.id] = at[unit.slot].distance_to(place) > lost
	if not far.values().has(true):
		return []  # nobody has fallen behind: nothing to cluster
	var out := []
	var kept := false
	for cluster in _clusters(placed, at):
		var behind: bool = cluster.all(func(u): return far[u.id])
		if behind:
			out.append_array(cluster)
		else:
			kept = true
	return out if kept else []


## The units in clusters: each unit within group_join of another of its cluster. Each
## unit looks only at its neighbours on a grid (BodyGrid), so it is linear in units.
static func _clusters(units: Array, points: PackedVector2Array) -> Array:
	if units.is_empty():
		return []
	var join := BattleTuning.current().group_join + 1.0
	var grid := BodyGrid.build(units.map(func(u): return points[u.slot]))
	var seen := PackedByteArray()
	seen.resize(units.size())
	var out := []
	for start in units.size():
		if seen[start] == 1:
			continue
		seen[start] = 1
		var cluster := [units[start]]
		var i := 0
		while i < cluster.size():
			var at: Vector2 = points[cluster[i].slot]
			for found in BodyGrid.near(grid, at, join + BodyGrid.MARGIN):
				if seen[found] == 0 and points[units[found].slot].distance_to(at) <= join:
					seen[found] = 1
					cluster.append(units[found])
			i += 1
		out.append(cluster)
	return out


## A new group of `cluster`, taken out of `squad`: the same command, route and orders.
## `at`: where the store's units stand, as read for the pass.
static func _group(
	squad: SkirmishSquad, cluster: Array, fight_seed: int, at: PackedVector2Array
) -> SkirmishSquad:
	var across := UnitMotion.vector(squad.heading).orthogonal()
	var keyed := cluster.map(
		func(u):
			return [snappedf(at[u.slot].dot(across), 0.0001), ScrumContest.draw(u, fight_seed), u]
	)
	keyed.sort_custom(func(a, b): return a.slice(0, 2) < b.slice(0, 2))
	var members: Array[SkirmishUnit] = []
	var front := -INF
	# store: scattered (few: a split, once as a group breaks off)
	for entry in keyed:
		var unit: SkirmishUnit = entry[2]
		squad.units.erase(unit)
		UnitState.regather(squad)
		SquadEntries.unloose(squad, unit.id)
		SquadEntries.unflee(squad, unit.id)
		UnitPlace.put(squad, unit, Vector2i(0, members.size()))
		members.append(unit)
		front = maxf(front, squad.route.distance_of(at[unit.slot]) * squad.direction)
	var group := SkirmishSquad.new(
		0,
		squad.faction_id,
		squad.direction,
		squad.home_distance,
		members.size(),
		members,
		squad.command,
		squad.store
	)
	group.route = squad.route
	group.front_distance = front * squad.direction / MapLayoutDef.CELLS_PER_TILE
	group.heading = squad.heading
	group.order = squad.order
	group.state = squad.state
	group.morale = squad.morale
	return group
