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
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
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
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationOrders = preload("res://sim/skirmish/formation/formation_orders.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const FormationSweep = preload("res://sim/skirmish/formation/formation_sweep.gd")
const FormationStaging = preload("res://sim/skirmish/formation/formation_staging.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
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
## The crate's first choices for the move (`moving::Choice`): it stands, it waits.
const MOVE_STANDS := 0
const MOVE_WAITS := 1

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
## Which squads march, hold or wait, and the landing, are the crate's (`moving.rs`, spec 39
## step 8b1); each march between, here.
func _move(events: Array, roster: Object) -> void:
	var ground := {"squads": _squads}  # the bodies lying on the field, found once (drag)
	var marching := []  # [mover, its route's end] a march, in the order planned
	var plans := PackedFloat64Array()  # [its index in the roster, where it wants to get] a march
	var waiting := []
	var choices: PackedInt32Array = _field.move_choices(roster)
	for listed in _squads.size():
		var mover := _squads[listed]
		if choices[listed] == MOVE_STANDS:
			continue
		if choices[listed] == MOVE_WAITS:
			waiting.append(mover)
			continue
		var paced := [TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE, tick_seconds]
		if FormationStaging.holds(mover, _squads, _tick, events, paced, terrain, roster):
			continue
		var end := FormationMarch.length(mover, route_length)
		if mover.order == SkirmishUnit.Order.RETREAT:  # home already: it holds, no about-face
			FormationMarch.check_ends(mover, end, _tick, events)
		if _field.move_holds(roster, listed):
			continue
		var travel := FormationMarch.travel_sign(mover, end)
		if travel != mover.direction and ScrumTurn.begin(mover, travel, _tick, events):
			continue  # an about-face is a re-form: its units walk to their places (Decision 92)
		var step := _step(mover, ground, roster, events)
		if step < 0.0:
			continue  # it waits for its units (spec 30 round 3)
		marching.append([mover, end])
		plans.append_array([listed, FormationMarch.toward(mover, travel * step, end)])
	_march(marching, plans, events, roster)
	for mover in waiting:
		mover.wait_ticks -= 1


## How far the squad steps this tick (tiles), sweeping round any bend as it goes; -1 if it
## waits for its units, out of its slack.
func _step(mover: SkirmishSquad, ground: Dictionary, roster: Object, events: Array) -> float:
	var keeping := FormationWalk.share(mover, terrain, fight_seed)  # within its units' slack
	if keeping <= 0.0:
		return -1.0
	var cells_per_second := mover.speed() * TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE
	FormationSweep.step(mover, cells_per_second, tick_seconds * keeping)  # Decision 105
	var open_step := mover.speed() * TRAVEL_SCALE * tick_seconds
	var step := FormationMarch.pace(mover, terrain, open_step, _tick, events)
	return step * GroundBodies.drag(mover, ground, roster) * keeping  # bodies on the ground


## Lands every move as the crate judged it (an advancing squad stopped at contact or behind
## a friend, as they stood at the tick's start); then arrivals, and waves that reached a
## friend join it.
func _march(marching: Array, plans: PackedFloat64Array, events: Array, roster: Object) -> void:
	var landed: PackedFloat64Array = _field.move_land(roster, plans)
	var joiners := []
	for n in marching.size():
		marching[n][0].front_distance = landed[2 * n]
		if landed[2 * n + 1] != 0.0:
			joiners.append(marching[n][0])
	for plan in marching:
		FormationMarch.check_ends(plan[0], plan[1], _tick, events)
	FormationJoins.join(_squads, joiners, _tick, fight_seed, events)


## The blows and shots chosen and landed, then their hurts landed in the crate (`hurts.rs`,
## spec 39 step 8b2), the fallen on the fell queue; their events, from what each left.
func _fight(events: Array) -> void:
	var interval := _attack_interval_ticks()
	var blows := FormationMelee.blows(_squads, interval, fight_seed, _field)  # targets cleared
	var shots := FormationCombat.ranged_blows(_squads, interval, fight_seed)
	var rolls := [fight_seed, _tick] if blow_rolls else []
	BlowLanding.land(blows, shots, _squads, terrain, rolls, _field)
	var hurts := []  # the shots, then the blows, in the order landed: hurt in the crate
	for each in shots + blows:
		hurts.append([each[1], each[2]])
	var left := UnitHealth.hurt_landed(self, hurts)
	for shot in shots:
		var spat := {"target": shot[1].id, "dmg": shot[2], "damage_type": shot[0].damage_type}
		spat["blow"] = shot[4]  # in the event after its damage type, as always
		events.append(FormationEvents.unit_event("spat", _tick, shot[3], shot[0], spat))
	for index in blows.size():
		events.append(FormationEvents.hit(_tick, blows[index], left[shots.size() + index]))
	FormationWounds.surrender(blows, _squads, fight_seed, _tick, events)
	FormationStamina.strike(self, blows + shots)


## Ticks in a second: a weapon's attack interval is in seconds (Decision 120).
func _attack_interval_ticks() -> int:
	return maxi(1, roundi(1.0 / tick_seconds))
