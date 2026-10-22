class_name RoutCatch
extends RefCounted
## Which friendly formation catches a router (Decisions 82, 89 and 98; spec 27 round 11):
## any standing friend a router has run into and got behind - the friend now between it and
## the enemy - holds it, wherever it then steps to, until it joins that formation (its rear)
## or the formation itself routs; the nearest friend catches, not the first
## listed, a tie going to the friends' units' seeded draws (Decision 97). Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const RoutFriends = preload("res://sim/skirmish/formation/rout_friends.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


## The formation holding a router (fleeing in its routing `squad`): the one
## that caught it while it stands, else the nearest friend within `reach` of `at`.
## `friends` is the tick's RoutFriends.
static func holder(
	squad: SkirmishSquad,
	unit: SkirmishUnit,
	at: Vector2,
	friends: Dictionary,
	reach: float,
	fight_seed: int = 0
) -> SkirmishSquad:
	var held_by = EntryKeys.held(squad.store.columns, unit.slot, UnitColumns.HELD_BY)
	for friend in RoutFriends.with_id(friends, held_by):
		if read_once(friends, friend, "stands"):
			return friend
	return friend_near(squad, at, friends, reach, false, fight_seed)


## The nearest standing friendly formation with a unit's body within `reach` of `at`
## (measured to the body's edge, Decision 106); if `led`, only one with a leader. Ties go
## by the units' draws. `friends` is the tick's RoutFriends.
static func friend_near(
	squad: SkirmishSquad,
	at: Vector2,
	friends: Dictionary,
	reach: float,
	led: bool,
	fight_seed: int = 0
) -> SkirmishSquad:
	var best := []  # [snapped gap, draw or null, unit, friend]
	for near in RoutFriends.around(friends, squad, at, reach, led):
		var gap: float = near[2].distance_to(at) - ScrumReach.radius(near[0])
		if gap > reach:
			continue
		var snapped := snappedf(gap, 0.000001)
		if (
			ScrumNear.beats(best, snapped, near[0], fight_seed)
			and _behind(squad, at, near[2], read_once(friends, squad, "home"))
		):
			best = [snapped, null, near[0], near[1]]
	return null if best.is_empty() else best[3]


## Cells from `at` to the formation's nearest living unit.
static func gap(squad: SkirmishSquad, at: Vector2) -> float:
	var least := INF
	for unit in squad.living():
		least = minf(least, ScrumReach.at(squad, unit).distance_to(at))
	return least


## True if a router at `at` has got behind a friend's unit at `there`: past it, on the
## side of its route towards home, so the friend stands between it and the enemy.
static func behind(squad: SkirmishSquad, at: Vector2, there: Vector2) -> bool:
	return _behind(squad, at, there, squad.home_distance * MapLayoutDef.CELLS_PER_TILE)


## `behind`, its squad's home (cells along its route) read once a pass (`read_once`).
static func _behind(squad: SkirmishSquad, at: Vector2, there: Vector2, home: float) -> bool:
	if squad.route == null:
		return true
	var mine: float = squad.route.distance_of(at)
	var theirs: float = squad.route.distance_of(there)
	return (mine - theirs) * (home - theirs) >= 0.0


## True if the formation is neither routing nor destroyed.
static func stands(squad: SkirmishSquad) -> bool:
	return not squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]


## What the rallies read of a squad - "home" (cells along its route), "stands" or "band"
## (FormationMorale's) - once for all its routers (Decision 144): kept in `friends["read"]`
## while the rallies are found, when nothing changes a squad; read afresh without it. A band
## is first read where a router first asks it, as before.
static func read_once(friends: Dictionary, squad: SkirmishSquad, what: String) -> Variant:
	var read: Dictionary = friends.get("read", {})
	var key := [squad.id, what]
	if not read.has(key):
		read[key] = _read(squad, what)
	return read[key]


static func _read(squad: SkirmishSquad, what: String) -> Variant:
	match what:
		"home":
			return squad.home_distance * MapLayoutDef.CELLS_PER_TILE
		"stands":
			return stands(squad)
	return FormationMorale.band(squad)
