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
## The breaking, the flight, the crush, the rallies, re-forming and reaching home are the
## crate's (`rout.rs`, spec 39 step 7b): this hands it the squads' roster (CrateRoster), the
## routes and each unit's leadership, and lands what it decides in its order - the hurts,
## the shocks (FormationMorale), the joins (FormationContact.reinforce) and the events. A
## routing squad's `fleeing`: unit id -> the unit; the cells along its route each has come
## are the store's ALONG, its spread from the route its FLEE_OFFSET (SquadEntries). Pure over
## the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
## The columns a router's point is read from (where_in).
const ROUTE := [UnitColumns.ALONG, UnitColumns.FLEE_OFFSET]
## The kinds of what the flight did (rout_flee's records).
const CRUSHED := 0
const HOME := 1
const SCATTERED := 2


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
	if not squads.any(func(s): return s.state == SkirmishSquad.State.ROUTING or s.morale == 0):
		return events  # nothing routs and nothing breaks
	var routing := []
	var routes := _routes(
		squads, func(squad): return squad.state == SkirmishSquad.State.ROUTING, routing
	)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var rules := _rules(cells_per_second * tick_seconds, tick_seconds)
	var ground: Object = UnitMover.ground(terrain)
	var roster := CrateRoster.of(squads, terrain)
	var leaders := _leaders(squads)
	var fled: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return field.rout_flee(roster, routes[0], routes[1], leaders, rules, fight_seed, ground)
	)
	var breaking := []
	for at in range(1, fled[0] + 1):
		breaking.append(squads[fled[at]])
	if _land_flight(fled.slice(fled[0] + 1), squads, routing, tick, events):
		roster = CrateRoster.of(squads, terrain)  # routers left the field
		leaders = _leaders(squads)
	if not routing.is_empty():
		if _rally(
			squads, routing, [field, roster, routes, leaders, rules, fight_seed], tick, events
		):
			roster = CrateRoster.of(squads, terrain)  # routers joined friends
			leaders = _leaders(squads)
		var reformed: PackedInt64Array = ColumnsLent.all(
			columns,
			func():
				return field.rout_regroup(roster, routes[0], routes[1], leaders, rules, fight_seed)
		)
		for at in reformed:
			events.append(FormationEvents.squad_event("reformed", tick, routing[at]))
	if not breaking.is_empty():
		_break(squads, breaking, [field, roster, ground], tick, events)
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


## [the crate's routing (each chosen squad's index in `squads` and its route's point count),
## the routes' points] for the squads `chosen` takes, which are appended to `out`.
static func _routes(squads: Array, chosen: Callable, out: Array) -> Array:
	var routing := PackedInt32Array()
	var points := PackedVector2Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if not chosen.call(squad):
			continue
		out.append(squad)
		var route := squad.route.points() if squad.route != null else PackedVector2Array()
		routing.append_array([index, route.size()])
		points.append_array(route)
	return [routing, points]


## Every listed squad's living units' leadership, in the roster's order.
static func _leaders(squads: Array) -> PackedInt32Array:
	var out := PackedInt32Array()
	for squad in squads:
		for unit in squad.living():
			out.append(unit.leadership)
	return out


## The crate's rout tuning (RoutTuning.of_slice), `pace` cells a tick at speed 1.
static func _rules(pace: float, tick_seconds: float) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			pace,
			tick_seconds,
			tuning.rout_fan_degrees,
			tuning.rout_fan_cells,
			tuning.rout_rally_reach,
			tuning.rout_steady_rally_seconds,
			tuning.rout_rally_seconds,
			tuning.rout_enemy_near,
			tuning.rout_reformed_morale,
			tuning.morale_steady_at
		]
	)


## Lands what the flight did (rout_flee's six numbers a record, in order): the crushes'
## hurts, events and panic, the routers home and the squads scattered. True if any router
## left the field.
static func _land_flight(
	fled: PackedInt64Array, squads: Array, routing: Array, tick: int, events: Array
) -> bool:
	var living := routing.map(func(squad): return squad.living())  # as the crate read them
	var left := false
	for at in range(0, fled.size(), 6):
		var squad: SkirmishSquad = routing[fled[at + 1]]
		match fled[at]:
			CRUSHED:
				var router: SkirmishUnit = living[fled[at + 1]][fled[at + 2]]
				var friend: SkirmishSquad = squads[fled[at + 3]]
				var hit: SkirmishUnit = friend.living()[fled[at + 4]]
				_crush(squad, router, [friend, hit, fled[at + 5] != 0], tick, events)
			HOME:
				var unit: SkirmishUnit = living[fled[at + 1]][fled[at + 2]]
				squad.units.erase(unit)
				UnitState.regather(squad)
				var extra := {"unit": unit.id, "definition": unit.definition}
				events.append(FormationEvents.squad_event("fled_home", tick, squad, extra))
				left = true
			SCATTERED:
				events.append(FormationEvents.squad_event("scattered", tick, squad))
	return left


## A router crushes a friend's unit (`hit` = [the friend, its unit, true if it panics]):
## both are hurt by its size, and a panicked friend takes the shock.
static func _crush(
	squad: SkirmishSquad, router: SkirmishUnit, hit: Array, tick: int, events: Array
) -> void:
	var damage := (
		BattleTuning.current().rout_crush * router.footprint_width * router.footprint_depth
	)
	UnitHealth.hurt_one(squad, router, damage)
	UnitHealth.hurt_one(squad, hit[1], damage)
	var extra := {"unit": router.id, "target": hit[1].id, "dmg": damage}
	events.append(FormationEvents.squad_event("crushed", tick, hit[0], extra))
	if hit[2]:
		FormationMorale.shock(hit[0], BattleTuning.current().rout_panic, tick, events)


## Decides the rallies in the crate (`call` = [field, roster, routes, leaders, rules, fight
## seed]) and makes the joins, nearest first. True if any router joined.
static func _rally(squads: Array, routing: Array, call: Array, tick: int, events: Array) -> bool:
	var morale := PackedInt64Array()
	for squad in squads:
		var unread: bool = squad.morale < 0
		morale.append_array([squad.morale, FormationMorale.ceiling(squad) if unread else 0])
	var field: Object = call[0]
	var columns: UnitColumns = squads[0].store.columns
	var rallied: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return field.rout_rallies(
				call[1], call[2][0], call[2][1], call[3], morale, call[4], call[5]
			)
	)
	for at in range(1, rallied[0] + 1):
		FormationMorale.band(squads[rallied[at]])  # first read: its morale is its ceiling
	var living := routing.map(func(squad): return squad.living())  # as the crate read them
	for at in range(rallied[0] + 1, rallied.size(), 3):
		var unit: SkirmishUnit = living[rallied[at]][rallied[at + 1]]
		_join(routing[rallied[at]], unit, squads[rallied[at + 2]], tick, events)
	return rallied.size() > rallied[0] + 1


## The breaking squads rout in the crate (`call` = [field, roster, ground]); each drops its
## staging and swaps, and its friends that see it break take the shock.
static func _break(squads: Array, breaking: Array, call: Array, tick: int, events: Array) -> void:
	var chosen := []
	var routes := _routes(squads, func(squad): return squad in breaking, chosen)
	var field: Object = call[0]
	var seen: PackedInt64Array = ColumnsLent.all(
		squads[0].store.columns,
		func(): return field.rout_break(call[1], routes[0], routes[1], call[2])
	)
	var at := 0
	for squad in chosen:
		squad.staging = {}
		squad.swaps.clear()
		events.append(FormationEvents.squad_event("routed", tick, squad))
		for index in range(at + 1, at + 1 + seen[at]):
			FormationMorale.shock(
				squads[seen[index]], BattleTuning.current().rout_seen, tick, events
			)
		at += 1 + seen[at]


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


static func _route_point(squad: SkirmishSquad, along: float) -> Vector2:
	if squad.route == null:
		return Vector2(along, 0.0)
	return squad.route.point_at(along)
