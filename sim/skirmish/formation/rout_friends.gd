class_name RoutFriends
extends RefCounted
## What a rout's routers look for near them - friends to crush, catch or flee to, enemies
## near - found through grids (BodyGrid, ScrumNear) rather than by looking at every unit on
## the field (spec 30 round 3). One tick's `friends` = {"squads": [...]} gathers each
## faction's index the first time it is asked for, and keeps it the rest of the tick:
## - "crush": the units of a faction's squads not routing, at their positions;
## - "stand": the units of its standing squads, where their bodies are (ScrumReach).
## Those squads stand where they are through a tick of routs; a unit killed since (crushed)
## is passed over. Each search picks as a search of every unit in the list's order would:
## the nearest by the distance snapped to 0.000001, then by the units' seeded draws
## (Decision 97), worked out only on a tie. Pure; cells.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## How far (cells) past the least (or most) a unit's reach across a route may lie and still
## be looked at for it: far above float rounding.
const ACROSS_MARGIN := 0.1


## [friend squad, its unit] for the friend's unit nearest a router of `squad` at `at`, one
## it runs into (nearer than a cell), ties by their draws; [] if none.
static func crushed(
	friends: Dictionary, squad: SkirmishSquad, at: Vector2, fight_seed: int
) -> Array:
	var index := _index(friends, "crush", squad.faction_id)
	if index["foes"].is_empty():
		return []
	var hit := []  # [snapped gap, draw or null, unit, friend]
	for found in BodyGrid.near(index["grid"], at, 1.0 + ScrumNear.MARGIN):
		var entry: Array = index["foes"][found]
		var gap: float = index["points"][found].distance_to(at)
		if gap >= 1.0 or entry[1] == squad or not entry[0].is_alive():
			continue
		var snapped := snappedf(gap, 0.000001)
		if ScrumNear.beats(hit, snapped, entry[0], fight_seed):
			hit = [snapped, null, entry[0], entry[1]]
	return [] if hit.is_empty() else [hit[3], hit[2]]


## [[unit, friend squad, where it stands], ...]: the units of `squad`'s standing friends
## whose bodies' edges may come within `reach` of `at`, in the list's order - every one
## that does; if `led`, only of friends with a leader.
static func around(
	friends: Dictionary, squad: SkirmishSquad, at: Vector2, reach: float, led: bool
) -> Array:
	var index := _index(friends, "stand", squad.faction_id)
	var out := []
	if index["foes"].is_empty():
		return out
	for found in BodyGrid.near(index["grid"], at, reach + index["widest"] + ScrumNear.MARGIN):
		var entry: Array = index["foes"][found]
		if entry[1] == squad or not entry[0].is_alive():
			continue
		if led and _leadership(friends, entry[1]) < 1:
			continue
		out.append([entry[0], entry[1], index["points"][found]])
	return out


## The standing friend of `squad` whose unit is nearest `at`, among those between the
## router and home on its route - `along` is [its distance along it, home's, the fight
## seed] - or null if there is none (RoutFlight's refuge).
static func refuge(friends: Dictionary, squad: SkirmishSquad, at: Vector2, along: Array):
	var index := _index(friends, "stand", squad.faction_id)
	if index["foes"].is_empty():
		return null
	var ahead := func(found: int) -> bool:
		var entry: Array = index["foes"][found]
		if entry[1] == squad or not entry[0].is_alive():
			return false
		var there: Vector2 = index["points"][found]
		return (squad.route.distance_of(there) - along[0]) * (along[1] - along[0]) > 0.0
	var found := ScrumNear.nearest(index, at, along[2], ahead)
	return null if found < 0 else index["foes"][found][1]


## How far across its route (along `side`) a router at `at` must turn aside to run into
## the friend's ranks: 0 if it is already heading into them. Each unit's reach across is
## (where it stands - at)·side, as ever, but worked out only for those at the edges.
static func short_of(
	friends: Dictionary, friend: SkirmishSquad, at: Vector2, side: Vector2
) -> float:
	var sorted := _across(friends, friend, side)
	var lowest: float = _edge(sorted, at, side, 1) - 0.5
	var highest: float = _edge(sorted, at, side, -1) + 0.5
	return clampf(0.0, lowest, highest)


## True if any enemy of `squad` stands within rout_enemy_near of any of `points` (its
## routers): each enemy looks through a grid of them, the first near enough answering.
static func enemy_near(friends: Dictionary, squad: SkirmishSquad, points: Array) -> bool:
	var grid := BodyGrid.build(points)
	var near := BattleTuning.current().rout_enemy_near
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for other in friends["squads"]:
		if other.faction_id == squad.faction_id:
			continue
		for enemy in other.living():
			for found in BodyGrid.near(grid, at[enemy.slot], near + ScrumNear.MARGIN):
				if at[enemy.slot].distance_to(points[found]) <= near:
					return true
	return false


## True if `squad` has no friend not routing, in any state: nothing for its routers to
## crush, flee to or be caught by.
static func alone(friends: Dictionary, squad: SkirmishSquad) -> bool:
	var kept: Dictionary = friends.get_or_add("alone", {})
	if not kept.has(squad.faction_id):
		kept[squad.faction_id] = not friends["squads"].any(
			func(other):
				return (
					other.faction_id == squad.faction_id
					and other.state != SkirmishSquad.State.ROUTING
				)
		)
	return kept[squad.faction_id]


## The squads listed with the id, in the list's order.
static func with_id(friends: Dictionary, id) -> Array:
	if not friends.has("by_id"):
		var by_id := {}
		for squad in friends["squads"]:
			by_id.get_or_add(squad.id, []).append(squad)
		friends["by_id"] = by_id
	return friends["by_id"].get(id, [])


## The index of `kind` for the faction (as ScrumNear.index: "foes" = [[unit, squad], ...],
## "points", "grid", "widest"), gathered the first time it is asked for.
static func _index(friends: Dictionary, kind: String, faction) -> Dictionary:
	var kept: Dictionary = friends.get_or_add(kind, {})
	if kept.has(faction):
		return kept[faction]
	var entries := []
	for other in friends["squads"]:
		var ours: bool = other.faction_id == faction
		var routing: bool = other.state == SkirmishSquad.State.ROUTING
		var counts: bool = ours and not routing
		if kind == "stand":
			counts = counts and other.state != SkirmishSquad.State.DESTROYED
		if counts:
			entries.append_array(other.living().map(func(unit): return [unit, other]))
	var index: Dictionary
	if kind == "stand":
		index = ScrumNear.index(entries)
	else:
		var at := PackedVector2Array()
		if not entries.is_empty():
			at = entries[0][1].store.columns.column(UnitColumns.POSITION)
		var points := entries.map(func(entry): return at[entry[0].slot])
		index = {"foes": entries, "points": points, "grid": BodyGrid.build(points)}
	kept[faction] = index
	return index


## A friend's leadership, kept for the tick: asked only once routers stop crushing.
static func _leadership(friends: Dictionary, friend: SkirmishSquad) -> int:
	var known: Dictionary = friends.get_or_add("leadership", {})
	if not known.has(friend):
		known[friend] = FormationMorale.leadership(friend)
	return known[friend]


## [[where·side, where, unit], ...] for the friend's living units, least first: kept for
## the tick, per friend and side.
static func _across(friends: Dictionary, friend: SkirmishSquad, side: Vector2) -> Array:
	var kept: Dictionary = friends.get_or_add("across", {}).get_or_add(friend, {})
	if not kept.has(side):
		var out := []
		for unit in friend.living():
			var there := ScrumReach.at(friend, unit)
			out.append([there.dot(side), there, unit])
		out.sort_custom(func(a, b): return a[0] < b[0])
		kept[side] = out
	return kept[side]


## The least (`way` 1) or most (-1) of (where - at)·side over the living units of `sorted`:
## only those within ACROSS_MARGIN of the edge can be it.
static func _edge(sorted: Array, at: Vector2, side: Vector2, way: int) -> float:
	var out := NAN
	var first := NAN
	var order := range(sorted.size()) if way == 1 else range(sorted.size() - 1, -1, -1)
	for index in order:
		var entry: Array = sorted[index]
		if not entry[2].is_alive():
			continue
		if is_nan(first):
			first = entry[0]
		elif (entry[0] - first) * way > ACROSS_MARGIN:
			break
		var across: float = (entry[1] - at).dot(side)
		if is_nan(out) or (across < out if way == 1 else across > out):
			out = across
	return out
