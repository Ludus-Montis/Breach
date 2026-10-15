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
## Squads keep `withdraw` ({"safe_ticks"}). Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const CELLS := float(MapLayoutDef.CELLS_PER_TILE)
## How near (cells, along its route) home a unit has nowhere further to go.
const HOME_CELLS := 0.5
## How far ahead along its route (cells) a fleeing unit looks for the way home.
const LOOK_CELLS := 1.0


## Starts the squad withdrawing: its units leave their places where they stand.
static func begin(squad: SkirmishSquad, tick: int, events: Array) -> void:
	squad.withdraw = {"safe_ticks": 0}
	squad.stance = {}
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if not squad.loose.has(unit.id):
			var stood := at[unit.slot]
			SquadEntries.loosen(squad, unit, {"unit": unit, "goal": null, "next": stood}, stood)
	events.append(FormationEvents.squad_event("withdrawing", tick, squad))


## How far from orderly the squad is, 0 (drilled) to 1 (no discipline): how wide it fans.
static func disorder(squad: SkirmishSquad) -> float:
	if FormationDiscipline.meets_threats(squad):
		return 0.0
	return (
		1.0 - float(FormationDiscipline.of(squad)) / BattleTuning.current().discipline_meets_threats
	)


## One tick of withdrawals: units flee, and squads that have been safe long enough re-form.
static func step(
	squads: Array,
	tick: int,
	pace: float,
	seconds: float,
	fight_seed: int,
	terrain: FormationTerrain,
	events: Array
) -> void:
	var withdrawing := []  # decided from where everyone stands, then moved (Decision 97)
	for squad in squads:
		if squad.withdraw.is_empty():
			continue
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			squad.withdraw = {}
			continue
		var foes := ScrumNear.index(ScrumBlows.hostile_units(squad, squads))
		var reach := BattleTuning.current().reach_contact
		var engaged := {}  # units still touching a foe: disengaging, not yet in flight
		for unit in squad.living():
			var at := ScrumReach.at(squad, unit)
			var close := ScrumNear.around(foes, at, ScrumReach.radius(unit) + reach)
			if ScrumBlows.touches_any(squad, unit, close):
				engaged[unit.id] = true
		withdrawing.append([squad, _safe(squad, squads, foes), engaged])
	for entry in withdrawing:
		var squad: SkirmishSquad = entry[0]
		var order := [disorder(squad), FormationDiscipline.reform_pace(squad)]  # its units'
		for unit in squad.living():
			var motion := [pace, seconds, fight_seed, entry[2].has(unit.id)] + order
			_flee(squad, unit, motion, terrain)
		squad.withdraw["safe_ticks"] = squad.withdraw["safe_ticks"] + 1 if entry[1] else 0
		var long_enough: bool = (
			squad.withdraw["safe_ticks"] * seconds >= BattleTuning.current().rout_rally_seconds
		)
		if long_enough or _home(squad):
			_reform_here(squad, tick, events)


## `motion` is [pace (cells a tick at speed 1), seconds, fight seed, still disengaging,
## its squad's disorder, its re-form pace].
## Disengaging is a manoeuvre, as re-forming is: until it is clear of the foes it touches,
## a unit turns and steps away at its formation's re-form pace; then it is in flight.
static func _flee(squad: SkirmishSquad, unit: SkirmishUnit, motion: Array, terrain) -> void:
	var entry: Dictionary = squad.loose[unit.id]
	var columns := squad.store.columns
	var at: Vector2 = columns.value(unit.slot, UnitColumns.LOOSE_AT)
	var along := squad.route.distance_of(at)
	var home: float = squad.home_distance * CELLS
	if absf(along - home) < HOME_CELLS:
		return  # home
	var homeward := _homeward(squad.route, along, home)
	var full: float = unit.speed * motion[0]
	var heading := DetMath.rotated(homeward, RoutFlight.fan(unit, motion[2]) * motion[4])
	var side := homeward.orthogonal()
	var aside: float = (at - squad.route.point_at(along)).dot(side)
	if (
		absf(aside) >= BattleTuning.current().rout_fan_cells * motion[4]
		and heading.dot(side) * aside > 0.0
	):
		heading = homeward  # fanned out as far as its disorder takes it (a rout's at most)
	var reach := maxf(full, 1.0)  # where its step lands, a run's longer than a cell
	if terrain != null and terrain.factor(unit, at, at + heading * reach) <= 0.0:
		var onto: Vector2 = squad.route.point_at(move_toward(along, home, LOOK_CELLS)) - at
		heading = onto.normalized() if onto.length() > 0.000001 else homeward  # back to the road
	var to := at + heading * maxf(full, 0.000001)
	var efficacy: float = motion[5]
	var turning: float = motion[1] * efficacy  # a turn is a re-form
	var step: float = full * minf(efficacy, 1.0) if motion[3] else full
	var stepped := UnitMover.step(unit, at, to, step, turning, terrain)  # pays the ground
	columns.put(unit.slot, UnitColumns.LOOSE_AT, stepped)
	entry["next"] = stepped
	entry["goal"] = null


## True if all its units are home: it has nowhere further to go.
static func _home(squad: SkirmishSquad) -> bool:
	var home: float = squad.home_distance * CELLS
	for unit in squad.living():
		if absf(squad.route.distance_of(ScrumReach.at(squad, unit)) - home) >= HOME_CELLS:
			return false
	return true


## The way home along the route from `along`: towards the route a step nearer home, so a
## unit at a bend turns with the route rather than running on along the leg it was on.
static func _homeward(route, along: float, home: float) -> Vector2:
	var nearer: float = move_toward(along, home, LOOK_CELLS)
	var way: Vector2 = route.point_at(nearer) - route.point_at(along)
	if way.length() < 0.000001:
		return route.heading_at(along) * signf(home - along)
	return way.normalized()


## True if no standing enemy is near the squad and none pursues it. `foes` is the index
## (ScrumNear) of its standing enemies.
static func _safe(squad: SkirmishSquad, squads: Array, foes: Dictionary) -> bool:
	for other in squads:
		if other.pursuit.get("foe") == squad.id and not other.pursuit.get("returning", false):
			return false
	var near := BattleTuning.current().rout_enemy_near
	for unit in squad.living():
		var at := ScrumReach.at(squad, unit)
		for found in BodyGrid.near(foes["grid"], at, near + ScrumNear.MARGIN):
			if foes["points"][found].distance_to(at) <= near:
				return false
	return true


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
