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
## Squads keep `withdraw` ({"safe_ticks"}), a view of their records (OrderRecords). Pure over
## the squads it is given. Its start is the crate's orders (FormationOrders); the flight, who
## is engaged, whether it is safe or home, its record and its re-forming are the crate's
## (`withdraw.rs`, spec 39 steps 4c and 7c): this hands it the squads' roster (CrateRoster),
## each withdrawing squad's disorder and re-form pace (FormationDiscipline), its route and
## home and its units' flight angles (RoutFlight.fan), and lands each re-form: its turn
## (ScrumTurn) and its events.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)


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
	if squads.all(func(squad): return squad.withdraw.is_empty()):
		return
	var listed := PackedInt32Array()
	var orders := PackedFloat64Array()  # per squad: disorder, re-form pace, home in cells
	var counts := PackedInt32Array()
	var points := PackedVector2Array()
	var fans := PackedFloat64Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		var ended: bool = (
			squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]
		)
		if squad.withdraw.is_empty() or ended:
			continue  # one routing or destroyed withdraws no more: the crate drops its record
		listed.append(index)
		var home: float = squad.home_distance * CELLS
		orders.append_array([disorder(squad), FormationDiscipline.reform_pace(squad), home])
		var route_points: PackedVector2Array = squad.route.points()
		counts.append(route_points.size())
		points.append_array(route_points)
		for unit in squad.living():
			fans.append(RoutFlight.fan(unit, fight_seed))
	if roster == null:
		roster = CrateRoster.of(squads, terrain)
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[
			pace,
			seconds,
			tuning.reach_contact,
			tuning.rout_enemy_near,
			tuning.rout_fan_cells,
			tuning.rout_rally_seconds
		]
	)
	var reformed: PackedInt64Array = squads[0].store.field().withdraw(
		roster, listed, orders, counts, points, fans, UnitMover.ground(terrain), rules
	)
	for at in range(0, reformed.size(), 3):
		_reformed(squads[reformed[at]], reformed[at + 1], reformed[at + 2] != 0, tick, events)


## The crate re-formed the squad on its route facing `homeward` (and, `arrived` home, made it
## hold there): its units walk to their places (a re-form), and it marches home on its order.
static func _reformed(
	squad: SkirmishSquad, homeward: int, arrived: bool, tick: int, events: Array
) -> void:
	ScrumTurn.begin(squad, homeward, tick, events)
	events.append(FormationEvents.squad_event("regrouping", tick, squad))
	if arrived:  # nowhere further to go: its retreat is done, and it holds facing out
		events.append(FormationEvents.squad_event("returned", tick, squad))
