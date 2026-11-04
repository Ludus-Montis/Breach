class_name FormationRout
extends RefCounted
## Rout (Decision 82, spec 27 round 3): a panicked flight, not an orderly about-face.
## - **Breaking:** at 0 morale a formation routs. Every lock on it ends, and friends that
##   see it break take a little shock.
## - **Flight:** its units flee one by one back along its route towards home at full
##   speed, fanning out from it as they go (RoutFlight, Decision 99). They strike nothing;
##   a hostile unit in contact strikes them from behind (FormationMelee, flank blows).
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
## The rout is one crate pass (`rout/pass.rs`, spec 39 steps 7b and 8b5-xxviii): it is handed
## the squads' roster (CrateRoster) and does the breaking, the flight, the crush, the
## rallies, re-forming and reaching home, landing the hurts, the shocks and the joins and
## logging the events; this mirrors GDScript's unit lists. A routing squad's `fleeing`: unit
## id -> the unit; the cells along its route each has come are the store's ALONG, its spread
## from the route its FLEE_OFFSET (SquadEntries). Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## The columns a router's point is read from (where_in).
const ROUTE := [UnitColumns.ALONG, UnitColumns.FLEE_OFFSET]


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
	if not squads.any(func(s): return s.state == SkirmishSquad.State.ROUTING or s.morale == 0):
		return []  # nothing routs and nothing breaks
	var store: UnitStore = squads[0].store
	var field: Object = ColumnsLent.field(store.columns)
	var tuning := rules(cells_per_second * tick_seconds, tick_seconds)
	var roster := CrateRoster.of(squads, terrain)
	var ground: Object = UnitMover.ground(terrain)
	var at := PackedInt64Array([tick, fight_seed])
	var moved: PackedInt64Array = ColumnsLent.all(
		store.columns,
		func(): return field.rout_step(roster, tuning, BandTuning.rules(), at, ground)
	)
	mirror(squads, moved)
	return CrateEvents.taken(field, store)


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


## The crate's rout tuning (RoutTuning.of_slice, then rout_crush, rout_panic and rout_seen),
## `pace` cells a tick at speed 1.
static func rules(pace: float, tick_seconds: float) -> PackedFloat64Array:
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
			tuning.morale_steady_at,
			tuning.rout_crush,
			tuning.rout_panic,
			tuning.rout_seen
		]
	)


## GDScript's lists after the crate's rout (`moved`: three numbers a router that left its
## squad, as `rout_step` returns them): one home is out of its squad's units; one that
## rallied is out of its squad's and the friend's, which took it in.
static func mirror(squads: Array, moved: PackedInt64Array) -> void:
	for at in range(0, moved.size(), 3):
		var squad: SkirmishSquad = squads[moved[at]]
		var unit: SkirmishUnit = null
		for member: SkirmishUnit in squad.units:
			if member.id == moved[at + 1]:
				unit = member
		squad.units.erase(unit)
		UnitState.regather(squad)
		if moved[at + 2] < 0:
			continue
		var leader: SkirmishSquad = squads[moved[at + 2]]
		FormationCommand.enlist(leader, unit)
		var joined := leader.joined.duplicate()
		joined.append(unit)
		leader.joined = joined


static func _route_point(squad: SkirmishSquad, along: float) -> Vector2:
	if squad.route == null:
		return Vector2(along, 0.0)
	return squad.route.point_at(along)
