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
const ScrumPursuit = preload("res://sim/skirmish/formation/scrum_pursuit.gd")
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
## States in which a squad doesn't march (a rout flees on its own: FormationRout).
const STANDING := [
	SkirmishSquad.State.DESTROYED,
	SkirmishSquad.State.FIGHTING,
	SkirmishSquad.State.ROUTING,
	SkirmishSquad.State.ARRIVED,
]

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
var _next_squad_id := 1
var _next_unit_id := 1
var _pending_orders := {}  # squad id -> SkirmishUnit.Order
var _tick := 0
var _field: Object = NativeKernels.body_field()  # the bodies, native or null (Decision 129)


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
	for placement in placements:
		members.append(
			FormationUnits.make(placement[0], placement[1], faction_id, direction, _next_unit_id)
		)
		_next_unit_id += 1
	var orders := FormationCommand.new(home, path)
	var squad := SkirmishSquad.new(
		_next_squad_id, faction_id, direction, home, width, members, orders, store
	)
	_next_squad_id += 1
	squad.route = path
	FormationMarch.face(squad)
	squad.wait_ticks = wait_ticks
	squad.morale = FormationMorale.ceiling(squad)
	squad.combat_width = combat_width
	_squads.append(squad)
	FormationMarch.sync_units([squad])  # placed: its units stand on their places
	return squad


## Takes effect on the next step(), whether or not the clock is paused now.
func order(squad_id: int, new_order: SkirmishUnit.Order) -> void:
	_pending_orders[squad_id] = new_order


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
	_engage(events)
	FormationPathing.step(_squads, terrain, _tick, fight_seed, events)
	_move(events)
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
	var fighting := {}  # before any lands, so orders given together act together
	for squad_id in _pending_orders:
		var given := squad(squad_id)
		var locked := given != null and given.engaged_with != 0
		fighting[squad_id] = locked or (given != null and not given.flank_contacts.is_empty())
	for squad_id in _pending_orders:
		var target := squad(squad_id)
		if target == null or target.state == SkirmishSquad.State.DESTROYED:
			continue
		target.order = _pending_orders[squad_id]
		var order_name: String = SkirmishUnit.Order.keys()[target.order]
		events.append(
			FormationEvents.squad_event("order_applied", _tick, target, {"order": order_name})
		)
		if target.order == SkirmishUnit.Order.RETREAT and fighting[squad_id]:
			events.append_array(
				ScrumPursuit.retreat(target, _squads, _tick, fight_seed, tick_seconds)
			)
	_pending_orders.clear()


func _engage(events: Array) -> void:
	perception = FormationPerception.see(_squads, terrain)  # all decide on one picture
	FormationFronts.engage(_squads, _tick, fight_seed, events)
	FormationEdges.engage(_squads, _tick, events, fight_seed)


## Each squad decides its move from the tick's start; all moves land together (Decision 97).
func _move(events: Array) -> void:
	var ground := {"squads": _squads}  # the bodies lying on the field, found once (drag)
	var marching := {}  # squad id -> [mover, where it wants to get, its route's end]
	var waiting := []
	for mover in _squads:
		if mover.state in STANDING or FormationScrum.regrouping(mover):
			continue
		if mover.wait_ticks > 0:
			waiting.append(mover)
			continue
		var paced := [TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE, tick_seconds]
		if FormationStaging.holds(mover, _squads, _tick, events, paced, terrain):
			continue
		var end := FormationMarch.length(mover, route_length)
		if mover.order == SkirmishUnit.Order.RETREAT:  # home already: it holds, no about-face
			FormationMarch.check_ends(mover, end, _tick, events)
		if mover.order == SkirmishUnit.Order.HOLD or FormationContact.skirmishing(mover, _squads):
			mover.state = SkirmishSquad.State.HOLDING
			continue
		mover.state = SkirmishSquad.State.MOVING
		var travel := FormationMarch.travel_sign(mover, end)
		if travel != mover.direction and ScrumTurn.begin(mover, travel, _tick, events):
			continue  # an about-face is a re-form: its units walk to their places (Decision 92)
		var keeping := FormationWalk.share(mover, terrain, fight_seed)  # within its units' slack
		if keeping <= 0.0:
			continue  # it waits for them (spec 30 round 3)
		var cells_per_second := mover.speed() * TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE
		FormationSweep.step(mover, cells_per_second, tick_seconds * keeping)  # Decision 105
		var open_step := mover.speed() * TRAVEL_SCALE * tick_seconds
		var step := FormationMarch.pace(mover, terrain, open_step, _tick, events)
		step *= GroundBodies.drag(mover, ground) * keeping  # bodies on the ground (Decision 121)
		marching[mover.id] = [mover, FormationMarch.toward(mover, travel * step, end), end]
	_march(marching, events)
	for mover in waiting:
		mover.wait_ticks -= 1


## Lands every move: an advancing squad stops at contact or behind a friend, as they stood
## at the tick's start; then arrivals, and waves that reached a friend join it.
func _march(marching: Dictionary, events: Array) -> void:
	var advancing := {}
	for squad_id in marching:
		if marching[squad_id][0].order == SkirmishUnit.Order.ADVANCE:
			advancing[squad_id] = true
	var landing := {}
	for squad_id in marching:
		var plan: Array = marching[squad_id]
		var next: float = plan[1]
		if advancing.has(squad_id):
			next = FormationContact.limit(plan[0], _squads, next, advancing)
		landing[plan[0]] = next
	for mover in landing:
		mover.front_distance = landing[mover]
	for squad_id in marching:
		FormationMarch.check_ends(marching[squad_id][0], marching[squad_id][2], _tick, events)
	var joiners := advancing.keys().map(func(squad_id): return marching[squad_id][0])
	FormationJoins.join(_squads, joiners, _tick, fight_seed, events)


func _fight(events: Array) -> void:
	var interval := _attack_interval_ticks()
	var blows := FormationMelee.blows(_squads, interval, fight_seed, _field)  # targets cleared
	var shots := FormationCombat.ranged_blows(_squads, interval, fight_seed)
	var rolls := [fight_seed, _tick] if blow_rolls else []
	BlowLanding.land(blows, shots, _squads, terrain, rolls, _field)
	for shot in shots:
		shot[1].hp -= shot[2]
		var spat := {
			"target": shot[1].id,
			"dmg": shot[2],
			"damage_type": shot[0].damage_type,
			"blow": shot[4]
		}
		events.append(FormationEvents.unit_event("spat", _tick, shot[3], shot[0], spat))
	for blow in blows:
		blow[1].hp -= blow[2]
		events.append(FormationEvents.hit(_tick, blow))
	FormationWounds.surrender(blows, _squads, fight_seed, _tick, events)
	FormationStamina.strike(blows + shots)


## Ticks in a second: a weapon's attack interval is in seconds (Decision 120).
func _attack_interval_ticks() -> int:
	return maxi(1, roundi(1.0 / tick_seconds))
