class_name FormationWithdraw
extends RefCounted
## A withdrawal (Decision 99, spec 27 round 10): a retreat is how a formation deals with an
## enemy, as fighting is, so it shares combat's priority (FormationManoeuvre). Ordered to
## retreat out of a fight, it leaves from where its units stand - no walking back to their
## places first:
## - **Disengaging, then flight:** each unit turns for home and heads there along its
##   route. Disengaging is a manoeuvre whose efficacy its formation's discipline sets, as
##   re-forming is (Decisions 92 and 101): until clear of the foes it touches, a unit turns
##   at its turn rate, and steps away at its pace, times the re-form pace - a disciplined
##   formation breaks off quickly and cleanly, a ragged one slowly, backing off and exposed
##   (flank blows) while it turns. Once clear it is in flight, at its full pace. Until it
##   has turned, a unit still facing a foe it touches strikes it (ScrumBlows). The less
##   ordered the formation, the wider its units fan out from the route's line: up to
##   rout_fan_degrees (BattleTuning) either side, by a seeded angle per unit, for one with no
##   discipline. Ground it can't cross turns it back onto the route (a ford it fanned
##   away from), and holds it only if that is barred too.
## - **Safe:** with no enemy within rout_enemy_near of it, and none pursuing it,
##   for rout_rally_seconds - the test a rout rallies by - it re-forms on its route
##   where its units stand, facing home, and marches home (its order). One whose units are
##   all home, with nowhere further to go, re-forms there at once, facing out the way it
##   will hold, and fights as any other. Its units fan out no further than a rout's, by its
##   disorder.
## Squads keep `withdraw` ({"safe_ticks"}). Pure over the squads it is given. The flight,
## who is engaged and whether it is safe or home are the crate's (`withdraw.rs`, spec 39 step
## 4c): this hands it the squads' roster (CrateRoster), each withdrawing squad's disorder and
## re-form pace (FormationDiscipline), its route and home, whether a squad pursues it, and
## its units' flight angles (RoutFlight.fan), and keeps its record and its re-forming.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)


## Starts the squad withdrawing: its units leave their places where they stand.
static func begin(squad: SkirmishSquad, tick: int, events: Array) -> void:
	squad.withdraw = {"safe_ticks": 0}
	squad.stance = {}
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if not squad.loose.has(unit.id):
			var stood := at[unit.slot]
			SquadEntries.loosen(squad, unit, stood)
	events.append(FormationEvents.squad_event("withdrawing", tick, squad))


## How far from orderly the squad is, 0 (drilled) to 1 (no discipline): how wide it fans.
static func disorder(squad: SkirmishSquad) -> float:
	if FormationDiscipline.meets_threats(squad):
		return 0.0
	return (
		1.0 - float(FormationDiscipline.of(squad)) / BattleTuning.current().discipline_meets_threats
	)


## One tick of withdrawals: units flee, and squads that have been safe long enough re-form.
## `roster`: the tick's (CrateRoster), made here if none is given.
static func step(
	squads: Array,
	tick: int,
	pace: float,
	seconds: float,
	fight_seed: int,
	terrain: FormationTerrain,
	events: Array,
	roster: Object = null
) -> void:
	var withdrawing := []
	var listed := PackedInt32Array()
	var pursued := PackedByteArray()
	var orders := PackedFloat64Array()  # per squad: disorder, re-form pace, home in cells
	var counts := PackedInt32Array()
	var points := PackedVector2Array()
	var fans := PackedFloat64Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if squad.withdraw.is_empty():
			continue
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			squad.withdraw = {}
			continue
		withdrawing.append(squad)
		listed.append(index)
		pursued.append(int(_pursued(squad, squads)))
		var home: float = squad.home_distance * CELLS
		orders.append_array([disorder(squad), FormationDiscipline.reform_pace(squad), home])
		var route_points: PackedVector2Array = squad.route.points()
		counts.append(route_points.size())
		points.append_array(route_points)
		for unit in squad.living():
			fans.append(RoutFlight.fan(unit, fight_seed))
	if withdrawing.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads, terrain)
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[pace, seconds, tuning.reach_contact, tuning.rout_enemy_near, tuning.rout_fan_cells]
	)
	var out: PackedByteArray = squads[0].store.field().withdraw(
		roster, listed, pursued, orders, counts, points, fans, UnitMover.ground(terrain), rules
	)
	for index in withdrawing.size():
		var squad: SkirmishSquad = withdrawing[index]
		var safe := out[index] & 1 != 0
		squad.withdraw["safe_ticks"] = squad.withdraw["safe_ticks"] + 1 if safe else 0
		var long_enough: bool = squad.withdraw["safe_ticks"] * seconds >= tuning.rout_rally_seconds
		if long_enough or out[index] & 2 != 0:
			_reform_here(squad, tick, events)


## True if all its units are home: it has nowhere further to go. The crate's
## (`withdraw::home`).
static func _home(squad: SkirmishSquad) -> bool:
	var home: float = squad.home_distance * CELLS
	var roster := CrateRoster.of([squad])
	return squad.store.field().withdraw_home(roster, 0, squad.route.points(), home)


## True if a squad pursues it, not yet on its way back.
static func _pursued(squad: SkirmishSquad, squads: Array) -> bool:
	for other in squads:
		if other.pursuit.get("foe") == squad.id and not other.pursuit.get("returning", false):
			return true
	return false


## Puts the squad's frame on its route where its units stand, facing home; its units walk
## to their places (a re-form) and it marches home on its order.
static func _reform_here(squad: SkirmishSquad, tick: int, events: Array) -> void:
	var total := 0.0
	var living := squad.living()
	for unit in living:
		total += squad.route.distance_of(ScrumReach.at(squad, unit))
	var mean := total / maxi(1, living.size()) / CELLS
	var lowest := minf(squad.home_distance, squad.front_distance)
	var highest := maxf(squad.home_distance, squad.front_distance)
	squad.front_distance = clampf(mean, lowest, highest)
	squad.withdraw = {}
	var homeward := -1 if squad.home_distance < squad.front_distance else 1
	var arrived := _home(squad)
	if arrived:  # nowhere further to go: its retreat is done, and it holds facing out
		homeward = 1 if is_zero_approx(squad.home_distance) else -1
		squad.front_distance = squad.home_distance
	ScrumTurn.begin(squad, homeward, tick, events)
	events.append(FormationEvents.squad_event("regrouping", tick, squad))
	if arrived:
		squad.order = SkirmishUnit.Order.HOLD
		events.append(FormationEvents.squad_event("returned", tick, squad))
