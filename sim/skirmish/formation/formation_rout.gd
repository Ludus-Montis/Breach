class_name FormationRout
extends RefCounted
## Rout (Decision 82, spec 27 round 3): a panicked flight, not an orderly about-face.
## - **Breaking:** at 0 morale a formation routs. Every lock on it ends, and friends that
##   see it break take a little shock.
## - **Flight:** its units flee one by one back along its route towards home at full
##   speed, fanning out from it as they go (RoutFlight, Decision 99). They strike nothing;
##   a hostile unit in contact strikes them from behind (RoutBlows, flank blows).
## - **Crush:** a router running through a friend's cell hurts both (blunt, by its size),
##   and that friend's formation takes panic shock.
## - **Rally:** a router that reaches a friendly formation with a leader joins its rear
##   ranks. One that runs into any standing friendly formation (Decisions 89 and 98) is
##   caught there: it stops (shoving in among them, its body kept apart: UnitBodies),
##   and after a few seconds with that formation steady (BattleTuning) joins its rear,
##   walking to its place. A shaken formation holds its routers until it steadies.
##   A routing formation whose own leader lives, with no enemy near for a few seconds,
##   re-forms where its leader stands and holds there.
## - **Home:** routers that reach home leave the field ("fled_home"; the player's routers
##   go back to the reserve).
## Nothing hangs on list order (Decision 97): who breaks is decided before any breaks, so
## one break's shock can't cascade within the tick for squads listed later; every router
## flees, then every rally is decided, then they join; a router crushes or is caught by the
## nearest friend; ties go to the units' seeded draws (ScrumContest).
## A routing squad's `fleeing`: unit id -> the unit; the cells along its route each has come
## are the store's ALONG, its spread from the route its FLEE_OFFSET (SquadEntries). Pure over
## the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const RoutCatch = preload("res://sim/skirmish/formation/rout_catch.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const RoutFriends = preload("res://sim/skirmish/formation/rout_friends.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
## The columns a router's point is read from (where_in).
const ROUTE := [UnitColumns.ALONG, UnitColumns.FLEE_OFFSET]
## The fleeing entries' columns, the keys that say which an entry holds (SquadEntries) and
## each router's pace (RoutFlight): the rout's view (UnitView), held for its pass.
const FLED := [UnitColumns.FANNED, UnitColumns.CAUGHT, UnitColumns.HELD_BY, UnitColumns.KEYS]
const HELD := ROUTE + FLED + [UnitColumns.SPEED]

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)


## One tick of routs: formations at 0 morale break, routers flee (crushing friends in the
## way), rally, re-form or reach home. `cells_per_second` is the pace at speed 1.
static func step(
	squads: Array,
	tick: int,
	cells_per_second: float,
	tick_seconds: float,
	terrain: FormationTerrain = null,
	fight_seed := 0
) -> Array:
	var events := []
	var view := UnitView.hold(squads, HELD)  # read router by router: held
	var breaking := squads.filter(
		func(s): return s.state != SkirmishSquad.State.ROUTING and _breaks(s)
	)
	var routing := squads.filter(func(s): return s.state == SkirmishSquad.State.ROUTING)
	var pace := cells_per_second * tick_seconds
	var friends := {"squads": squads}  # each faction's units, found by grids (RoutFriends)
	for squad in routing:
		_flee(squad, tick, [pace, fight_seed, terrain, friends], events)
	var joins := []
	friends["read"] = {}  # squads' fields, read once for the rallies (RoutCatch.read_once)
	for squad in routing:
		joins.append_array(_rallies(squad, friends, tick_seconds, fight_seed))
	friends.erase("read")  # the joins change squads
	joins.sort_custom(func(a, b): return a[0] < b[0])
	for entry in joins:
		_join(entry[1], entry[2], entry[3], tick, events)
	for squad in routing:
		_regroup(squad, friends, tick, tick_seconds, events, fight_seed)
	for squad in breaking:
		_break(squad, squads, tick, events, terrain)
	UnitView.release(view)
	return events


## Where a router stands, from a view holding ROUTE (UnitView): an index, not a call.
static func where_in(view: UnitView, squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return _route_point(squad, view.along[unit.slot]) + view.offset[unit.slot]


## The world point a router stands at: one router, scattered.
static func where(squad: SkirmishSquad, unit_id: int) -> Vector2:
	var slot: int = squad.fleeing[unit_id].slot
	var columns := squad.store.columns
	var along: float = columns.value(slot, UnitColumns.ALONG)
	return _route_point(squad, along) + columns.value(slot, UnitColumns.FLEE_OFFSET)


## A router's point on its route, before its offset (where_in is it plus the offset), from a
## view holding ALONG: an index, not a call.
static func route_point_in(view: UnitView, squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return _route_point(squad, view.along[unit.slot])


static func _breaks(squad: SkirmishSquad) -> bool:
	return (
		squad.state != SkirmishSquad.State.DESTROYED
		and squad.state != SkirmishSquad.State.ARRIVED
		and not squad.living().is_empty()
		and squad.morale == 0
	)


static func _break(
	squad: SkirmishSquad, squads: Array, tick: int, events: Array, terrain: FormationTerrain
) -> void:
	FormationLocks.release(squad, squads)
	squad.unflank_all()
	squad.staging = {}
	squad.swaps.clear()
	squad.state = SkirmishSquad.State.ROUTING
	squad.rally_ticks = 0
	var along := squad.front_distance * CELLS
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		var offset: Vector2 = at[unit.slot] - _route_point(squad, along)
		SquadEntries.flee(squad, unit, along, offset)
	events.append(FormationEvents.squad_event("routed", tick, squad))
	for friend in squads:
		if friend != squad and friend.faction_id == squad.faction_id and not friend.is_destroyed():
			if FormationSight.detects(friend, squad, terrain):
				FormationMorale.shock(friend, BattleTuning.current().rout_seen, tick, events)


## `motion` is [pace (cells a tick at speed 1), fight seed, terrain or null, the tick's
## RoutFriends].
static func _flee(squad: SkirmishSquad, tick: int, motion: Array, events: Array) -> void:
	var home := squad.home_distance * CELLS
	var panicked := {}
	var alone := RoutFriends.alone(motion[3], squad)  # no friend to crush or flee to
	var friends: Dictionary = {} if alone else motion[3]
	var view := UnitView.of(squad)  # the held view: indexed, not called
	var along: PackedFloat64Array = view.along
	var caught: PackedInt64Array = view.caught
	for unit in squad.living():
		if caught[unit.slot] > 0:
			continue  # held by a steady friend it ran into
		var at := where_in(view, squad, unit)
		RoutFlight.step(squad, unit, home, [motion[0], motion[1], motion[2], at, friends, view])
		if not alone:
			_crush([squad, unit, motion[1], view], friends, tick, panicked, events)
		if is_equal_approx(along[unit.slot], home):
			SquadEntries.unflee(squad, unit.id)
			squad.units.erase(unit)
			UnitState.regather(squad)
			var extra := {"unit": unit.id, "definition": unit.definition}
			events.append(FormationEvents.squad_event("fled_home", tick, squad, extra))
	if squad.living().is_empty():
		squad.state = SkirmishSquad.State.DESTROYED
		events.append(FormationEvents.squad_event("scattered", tick, squad))


## A router (`who` = [its squad, the unit, the battle seed, the rout's view]) crushes the
## nearest friend's unit it runs into (ties by their draws). `friends` is the tick's
## RoutFriends.
static func _crush(
	who: Array, friends: Dictionary, tick: int, panicked: Dictionary, events: Array
) -> void:
	var squad: SkirmishSquad = who[0]
	var router: SkirmishUnit = who[1]
	var at := where_in(who[3], squad, router)
	var hit := RoutFriends.crushed(friends, squad, at, who[2])  # [friend, unit]
	if hit.is_empty():
		return
	var damage := (
		BattleTuning.current().rout_crush * router.footprint_width * router.footprint_depth
	)
	UnitHealth.hurt_one(squad, router, damage)
	UnitHealth.hurt_one(squad, hit[1], damage)
	var extra := {"unit": router.id, "target": hit[1].id, "dmg": damage}
	events.append(FormationEvents.squad_event("crushed", tick, hit[0], extra))
	if not panicked.has(hit[0].id):
		panicked[hit[0].id] = true
		FormationMorale.shock(hit[0], BattleTuning.current().rout_panic, tick, events)


## [[key, squad, unit, friend], ...]: the squad's routers that join a friend this tick,
## decided before any joins - one with a leader near, or one held long enough by a steady
## friend - keyed nearest first. Counts each held router's time caught.
static func _rallies(
	squad: SkirmishSquad, friends: Dictionary, tick_seconds: float, fight_seed: int
) -> Array:
	var out := []
	var alone := RoutFriends.alone(friends, squad)  # no friend to rally to or be held by
	var view := UnitView.of(squad)  # the held view: indexed, not called
	var caught_for: PackedInt64Array = view.caught
	for unit in squad.living():
		var at := where_in(view, squad, unit)
		var leader: SkirmishSquad = null
		if not alone:
			var reach := BattleTuning.current().rout_rally_reach
			leader = RoutCatch.friend_near(squad, at, friends, reach, true, fight_seed)
		if leader != null:
			out.append([_key(leader, unit, at, fight_seed), squad, unit, leader])
			continue
		var breadth := 2.0 * ScrumReach.radius(unit)  # its own body's breadth
		var friend: SkirmishSquad = null
		if not alone:
			friend = RoutCatch.holder(squad, unit, at, friends, breadth, fight_seed)
		var calm: bool = (
			friend != null
			and RoutCatch.read_once(friends, friend, "band") == FormationMorale.Band.STEADY
		)
		var held: int = caught_for[unit.slot]
		var caught := 0 if friend == null else (held + 1 if calm else maxi(held, 1))
		view.keep(unit.slot, UnitColumns.CAUGHT, caught)
		view.keep(unit.slot, UnitColumns.HELD_BY, null if friend == null else friend.id)
		if calm and caught * tick_seconds >= BattleTuning.current().rout_steady_rally_seconds:
			out.append([_key(friend, unit, at, fight_seed), squad, unit, friend])
	return out


## Routers joining a friend line up behind it nearest first, ties by their draws.
static func _key(friend: SkirmishSquad, unit: SkirmishUnit, at: Vector2, fight_seed: int) -> Array:
	return [snappedf(RoutCatch.gap(friend, at), 0.000001), ScrumContest.draw(unit, fight_seed)]


## A routing formation whose own leader lives re-forms after a while with no enemy near.
## `friends` is the tick's RoutFriends.
static func _regroup(
	squad: SkirmishSquad,
	friends: Dictionary,
	tick: int,
	tick_seconds: float,
	events: Array,
	fight_seed: int
) -> void:
	if squad.living().is_empty() or FormationMorale.leadership(squad) == 0:
		return
	var points := squad.living().map(func(unit): return where(squad, unit.id))
	var near := RoutFriends.enemy_near(friends, squad, points)
	squad.rally_ticks = 0 if near else squad.rally_ticks + 1
	if squad.rally_ticks * tick_seconds >= BattleTuning.current().rout_rally_seconds:
		_reform(squad, tick, events, fight_seed)


static func _join(
	squad: SkirmishSquad, unit: SkirmishUnit, leader: SkirmishSquad, tick: int, events: Array
) -> void:
	var settled := where(squad, unit.id)
	SquadEntries.unflee(squad, unit.id)
	squad.units.erase(unit)
	UnitState.regather(squad)
	UnitPlace.put(squad, unit, Vector2i.ZERO)
	var members: Array[SkirmishUnit] = [unit]
	var single := SkirmishSquad.new(
		-1, squad.faction_id, leader.direction, 0.0, 1, members, null, squad.store
	)
	FormationContact.reinforce(leader, single)
	leader.reforming = true
	SquadEntries.loosen(leader, unit, settled)
	events.append(FormationEvents.unit_event("rallied", tick, squad, unit, {"into": leader.id}))
	if squad.living().is_empty():
		squad.state = SkirmishSquad.State.DESTROYED


## Re-forms where its leader stands (the best, ties by draw), facing the enemy's end again,
## and holds.
static func _reform(squad: SkirmishSquad, tick: int, events: Array, fight_seed: int) -> void:
	var leader: SkirmishUnit = null
	var best := []
	for unit in squad.living():
		var key := [-unit.leadership, ScrumContest.draw(unit, fight_seed)]
		if leader == null or key < best:
			leader = unit
			best = key
	squad.front_distance = squad.store.columns.value(leader.slot, UnitColumns.ALONG) / CELLS
	SquadEntries.unflee_all(squad)
	squad.direction = 1 if is_zero_approx(squad.home_distance) else -1
	var way := Vector2.RIGHT * squad.direction  # it faces the enemy's end along its route
	if squad.route != null:
		way = squad.route.heading_at(squad.front_distance * CELLS) * squad.direction
	squad.heading = UnitMotion.bearing_to(Vector2.ZERO, way, squad.heading)
	squad.order = SkirmishUnit.Order.HOLD
	squad.state = SkirmishSquad.State.HOLDING
	squad.morale = BattleTuning.current().rout_reformed_morale
	squad.reforming = true
	events.append(FormationEvents.squad_event("reformed", tick, squad))


static func _route_point(squad: SkirmishSquad, along: float) -> Vector2:
	if squad.route == null:
		return Vector2(along, 0.0)
	return squad.route.point_at(along)
