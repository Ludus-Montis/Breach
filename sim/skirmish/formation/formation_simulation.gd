class_name FormationSimulation
extends RefCounted
## The formation fight (specs/22-formation-feel-test.md, Decision 40): squads of both
## factions, each on its route (spec 27). Deterministic, and no outcome hangs on the order
## squads were spawned or are listed in (Decision 97): each phase decides from one
## snapshot, then applies; ties go to where things stand, then to the seeded draws.
##
## step() runs one tick in this fixed order:
##   1. orders  - queued wave orders apply (a retreat disengages at once)
##   2. engage  - hostile squads whose fronts are within MELEE_REACH lock together, and a
##                free squad whose front reaches a hostile's side or rear locks onto that
##                edge (FormationEdges, Decision 78)
##   3. move    - a squad going back the way it faces about-faces first, a re-form
##                (ScrumTurn); a staged one holds until its trigger (Decision 87); free
##                squads move together as blocks at their slowest unit's pace, sweeping
##                round bends (FormationSweep, Decision 105), stopping at contact or behind
##                where a friend stood, or join a friend from the back (Decisions 44, 51,
##                FormationJoins); then units seek contact (FormationScrum, Decision 88)
##   4. combat  - melee blows (FormationMelee) and ranged blows (Decision 46), each rolled
##                against its target (BlowLanding, Decision 118)
##   5. deaths  - the fallen are downed or die, the ranks behind step up, an empty squad is
##                destroyed; the downed nobody guards are finished or taken (FormationWounds),
##                the rest come to or die of their wounds (FormationRecovery, Decision 126)
##   6. bodies  - overlapping friends' bodies are pushed apart by mass (UnitBodies, Decision 106)
##   7. re-form - units swap toward their preferred places (FormationShuffle, Decision 46)

const FormationTaking = preload("res://sim/skirmish/formation/formation_taking.gd")
const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationEdges = preload("res://sim/skirmish/formation/formation_edges.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationOrders = preload("res://sim/skirmish/formation/formation_orders.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const FormationStaging = preload("res://sim/skirmish/formation/formation_staging.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationFronts = preload("res://sim/skirmish/formation/formation_fronts.gd")
const FormationJoins = preload("res://sim/skirmish/formation/formation_joins.gd")
const FormationPerception = preload("res://sim/skirmish/formation/formation_perception.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const MELEE_REACH := FormationContact.MELEE_REACH
const TRAVEL_SCALE := 0.125  # tiles per second at speed 1: 8 cells a second (Decision 68)

var route_length: float
var tick_seconds: float
var combat_width := 0  # the lane's width later squads may spread to (Decision 51); 0: own columns
var fight_seed := 0  # the battle seed (Decision 93): every draw, for cells or blows, is from it
## Whether each blow is rolled against parry, dodge and defence, its damage in its weapons'
## range (Decisions 118 and 119, BlowLanding); off, each lands plainly at full damage.
var blow_rolls := false
var terrain: FormationTerrain = null  # the ground (Decision 85); null: open, level ground
var perception := {}  # what each formation sees as the tick begins (FormationPerception)
var store: UnitStore  # every unit's slot (spec 35), on the engine's backend; read, not replaced

var _route: FormationRoute
var _squads: Array[SkirmishSquad] = []
var _tick := 0
var _field: Object = NativeKernels.body_field()  # the bodies, native (Decision 129)
## The next squad id: the crate issues them (spec 39 step 8b4-vii); the strays and the groups
## number theirs from it and hand it back.
var _next_squad_id: int:
	get:
		return _field.squads_issued() + 1
	set(value):
		_field.put_squads_issued(value - 1)


func _init(length: float, seconds_per_tick: float) -> void:
	route_length = length
	tick_seconds = seconds_per_tick
	store = NativeKernels.unit_store(_field)
	_route = FormationRoute.straight(length * MapLayoutDef.CELLS_PER_TILE)


## placements: [[UnitDef, Vector2i(rank, column)], ...] from a WaveTemplate layout. The
## squad follows `route` (Decision 75), or the lane's straight route if null.
func spawn_squad(
	width: int,
	placements: Array,
	faction_id: String,
	at_player_end: bool,
	wait_ticks: int = 0,
	route: FormationRoute = null
) -> SkirmishSquad:
	var path := route if route != null else _route
	var direction := 1 if at_player_end else -1
	var home := 0.0 if at_player_end else path.length_cells() / MapLayoutDef.CELLS_PER_TILE
	var members: Array[SkirmishUnit] = []
	var unit_id: int = _field.issue_units(placements.size())  # the crate issues the ids
	for placement in placements:
		members.append(FormationUnits.make(placement[0], faction_id, direction, unit_id))
		unit_id += 1
	var orders := FormationCommand.new(_field, home, path)
	var squad_id: int = _field.issue_squad()
	var squad := SkirmishSquad.new(
		squad_id, faction_id, direction, home, width, members, orders, store
	)
	FormationUnits.enter(squad, members, placements)
	squad.route = path
	FormationMarch.face(squad)
	squad.wait_ticks = wait_ticks
	squad.morale = FormationMorale.ceiling(squad)
	squad.combat_width = combat_width
	_squads.append(squad)
	FormationMarch.sync_units([squad])  # placed: its units stand on their places
	return squad


## Takes effect on the next step(), whether or not the clock is paused now. The crate queues
## it (`orders::queue`): a second order for the squad before then replaces the first.
func order(squad_id: int, new_order: SkirmishUnit.Order) -> void:
	_field.order_give(squad_id, new_order)


func squads() -> Array[SkirmishSquad]:
	return _squads


func squad(squad_id: int) -> SkirmishSquad:
	for entry in _squads:
		if entry.id == squad_id:
			return entry
	return null


func tick_number() -> int:
	return _tick


func step() -> Array:
	_tick += 1
	var events := []
	_apply_orders(events)
	var roster := _engage(events)  # who is living, till the joins: nobody falls before them
	FormationPathing.step(_squads, terrain, _tick, fight_seed, events)
	_move(events, roster)
	var pace := TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE
	events.append_array(
		FormationScrum.step(_squads, _tick, pace, tick_seconds, fight_seed, terrain, _field)
	)
	_fight(events)
	FormationDeaths.bury(_squads, _tick, events)
	FormationTaking.step(_squads, _tick, tick_seconds, fight_seed, events, perception)
	FormationWounds.tend(_squads, _tick, _attack_interval_ticks(), fight_seed, events)
	var strays := FormationRecovery.step(_squads, tick_seconds, _tick, fight_seed, events)
	_next_squad_id = FormationStrays.adopt(strays, _squads, _next_squad_id, fight_seed)
	FormationMorale.step(_squads, _tick, _attack_interval_ticks(), events)
	events.append_array(FormationRout.step(_squads, _tick, pace, tick_seconds, terrain, fight_seed))
	strays = FormationCarry.step(_squads, _tick, fight_seed, events)  # the wounded borne
	_next_squad_id = FormationStrays.adopt(strays, _squads, _next_squad_id, fight_seed)
	_next_squad_id = FormationGroups.step(_squads, _tick, fight_seed, events, _next_squad_id)
	FormationStamina.step(_squads, tick_seconds)  # runners tire, the rest recover
	UnitBodies.step(_squads, fight_seed, _field)  # after every move: bodies part (Decision 106)
	for entry in _squads:
		events.append_array(FormationShuffle.step(entry, tick_seconds, TRAVEL_SCALE, _tick))
	FormationMarch.sync_units(_squads, [tick_seconds, pace], terrain, fight_seed)  # to places
	store.settle(_squads)  # the units no list holds any more give up their slots
	return events


func _apply_orders(events: Array) -> void:
	FormationOrders.apply(_squads, _field.orders_pending(), _tick, fight_seed, events)
	_field.orders_clear()  # taken, or nobody was left to take them


## The squads' roster (CrateRoster) it decided on, for the march's decisions too.
func _engage(events: Array) -> Object:
	var roster := CrateRoster.of(_squads)
	perception = FormationPerception.see(_squads, terrain, roster)  # all decide on one picture
	FormationFronts.engage(_squads, _tick, fight_seed, events, roster)
	FormationEdges.engage(_squads, _tick, events, fight_seed, roster)
	return roster


## Each squad decides its move from the tick's start; all moves land together (Decision 97).
## The move is the crate's (`moving::pass`, spec 39 step 8b5-viii): staging, the march's
## ends, the turn, the sweep, the pace, the drag and the landing, its events in the crate's
## log, and each squad's share of its pace (`moving::share`, 8b5-ix); the joins after
## (`joins.rs`, 8b5-x), whose dropped squads leave GDScript's list here.
func _move(events: Array, roster: Object) -> void:
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[
			TRAVEL_SCALE,
			tick_seconds,
			route_length,
			tuning.discipline_per_leadership,
			tuning.walk_slack_loose,
			tuning.walk_slack_drilled,
			tuning.bodies_ground_drag,
			tuning.bodies_ground_block,
			tuning.wounds_reach,
			tuning.walk_lost,
		]
	)
	var codes := FormationStaging.route_codes(_squads)
	var ground := UnitMover.ground(terrain)
	var joins: PackedInt32Array = _field.move_step(roster, codes, ground, _tick, fight_seed, rules)
	events.append_array(CrateEvents.taken(_field))
	FormationJoins.join(_squads, joins)


## The fight as one crate pass (FormationMelee.fight, spec 39 step 8b5-xix): the blows and
## shots chosen and landed, their hurts, the fallen on the fell queue, and the surrender
## (8b5-xx), whose takings GDScript's copies mirror; then its events.
func _fight(events: Array) -> void:
	var ticks := [_attack_interval_ticks(), fight_seed, _tick]  # FormationMelee.fight's order
	for taken in FormationMelee.fight(_squads, ticks, blow_rolls, terrain, _field):
		UnitState.put(taken[0], taken[1], SkirmishUnit.State.TAKEN)
	events.append_array(CrateEvents.taken(_field))  # "spat", "hit", "surrendered", "destroyed"


## Ticks in a second: a weapon's attack interval is in seconds (Decision 120).
func _attack_interval_ticks() -> int:
	return maxi(1, roundi(1.0 / tick_seconds))
