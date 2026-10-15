class_name FormationWalk
extends RefCounted
## Units walk to their places (spec 30 round 3, part 2): a formation's frame is a set of
## places, not positions. Every unit in its place - not loose in a fight, nor fleeing -
## walks to it each tick at its own pace on the ground it crosses, as loose units do: a
## wheel's outer file walks the arc, a re-form walks, a unit pushed aside walks back. Far
## from its place it faces where it walks; near it, its formation's way, stepping sideways
## or back. The frame keeps within a slack of its units (share): it slows as a unit lags
## towards what its formation's discipline allows (walk_slack_drilled to walk_slack_loose)
## and waits beyond it, unless that unit has fallen behind (walk_lost) - then only "no man
## left behind" waits for it; under "fall behind, left behind" it never waits. Placing a
## squad sets its units on their places (FormationMarch.sync_units). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const WalkModes = preload("res://sim/skirmish/formation/walk_modes.gd")
const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const TerrainStep = preload("res://sim/skirmish/formation/terrain_step.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationPour = preload("res://sim/skirmish/formation/formation_pour.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001


## Walks `unit` of `squad` a tick towards `place`; `timing` is [seconds, cells a second
## at speed 1 on open ground, and optionally the bodies lying on the field], on `terrain`
## (null: open).
static func walk(
	squad: SkirmishSquad,
	unit: SkirmishUnit,
	place: Vector2,
	timing: Array,
	terrain: FormationTerrain = null
) -> void:
	var seconds: float = timing[0]
	var columns := squad.store.columns
	if WalkModes.climb(unit, columns, terrain, seconds, unit.speed * timing[1]):
		return  # on a climb face: it climbs before it moves on
	var from: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
	var going := TerrainStep.way(unit, from, place, terrain)  # its way round a barrier
	var toward: Vector2 = going[0]
	var way := toward - from
	var full: float = unit.speed * timing[1] * seconds
	if not WalkModes.may_step(unit, from, toward, terrain):
		full = 0.0  # spent: it won't start a climb or a swim
	if timing.size() > 2 and way.length() > EPSILON:  # bodies on the ground (GroundBodies)
		full *= GroundBodies.underfoot(unit, from + way.normalized() * 0.5, timing[2])
	var facing := UnitMotion.vector(squad.heading)
	var far := place.distance_to(from) > _tuning().walk_face_travel
	var look := toward if far else place + facing
	var stepped := UnitMover.step(unit, from, toward, full, seconds, terrain, look)
	columns.put(unit.slot, UnitColumns.POSITION, stepped)
	WalkModes.after_step(unit, columns, from, terrain, seconds)


## Where `unit` belongs in its squad's frame now, including any swap under way (`moving`:
## FormationShuffle.movers for the squad, if the caller has it).
static func place_of(squad: SkirmishSquad, unit: SkirmishUnit, moving = null) -> Vector2:
	return FormationPour.place(squad, unit, moving)


## The share (0 to 1) of its step the squad's frame takes this tick for its units lagging
## behind their places: all of it while every unit is within half its slack, easing to
## none as the furthest nears the slack itself, so a wheel slows to what its outer file
## can walk rather than halting; one fallen behind (walk_lost) is waited for only if no
## man is left behind.
static func share(
	squad: SkirmishSquad, terrain: FormationTerrain = null, fight_seed: int = 0
) -> float:
	if not squad.taking.is_empty():
		return 0.0  # it holds while its units are out taking the downed (FormationTaking)
	var rule := rule_of(squad)
	if rule == "fall_behind_left_behind":
		return 1.0
	var slack := slack_of(squad)
	var furthest := 0.0
	var moving := FormationShuffle.movers(squad)
	var aims := FormationPour.targets(squad, terrain, moving, fight_seed)
	var points: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if squad.loose.has(unit.id) or squad.fleeing.has(unit.id):
			continue
		var target: Vector2 = aims[unit.id]
		if not target.is_equal_approx(place_of(squad, unit, moving)):
			continue  # its place is barred: it pours through the gap behind the frame
		var lag := points[unit.slot].distance_to(target)
		if lag <= _tuning().walk_lost or rule == "no_man_left_behind":
			furthest = maxf(furthest, lag)
	return clampf((slack - furthest) / (slack / 2.0), 0.0, 1.0)


## Where `unit` makes for (FormationPour.targets): its place, or where its place is barred,
## the nearest point it can walk, filed in behind those squeezed there before it.
static func target_of(
	squad: SkirmishSquad,
	unit: SkirmishUnit,
	terrain: FormationTerrain = null,
	moving = null,
	fight_seed: int = 0
) -> Vector2:
	var loose := squad.loose.has(unit.id)
	return FormationPour.targets(squad, terrain, moving, fight_seed, loose)[unit.id]


## True if the squad's frame waits this tick for a unit lagging behind its place.
static func waits(squad: SkirmishSquad, terrain: FormationTerrain = null) -> bool:
	return share(squad, terrain) <= EPSILON


## How far (cells) a unit of the squad may lag behind its place before the frame waits.
static func slack_of(squad: SkirmishSquad) -> float:
	var tuning := _tuning()
	var drilled := clampf(FormationDiscipline.of(squad) / 100.0, 0.0, 1.0)
	return lerpf(tuning.walk_slack_loose, tuning.walk_slack_drilled, drilled)


## "fall_behind_left_behind", "no_man_left_behind" or "": a leader's trait, or one every
## unit has (the never-wait wins if both are found).
static func rule_of(squad: SkirmishSquad) -> String:
	var living := squad.living()
	for trait_id in ["fall_behind_left_behind", "no_man_left_behind"]:
		var led := living.any(func(u): return u.leadership > 0 and u.traits.has(trait_id))
		var all := not living.is_empty() and living.all(func(u): return u.traits.has(trait_id))
		if led or all:
			return trait_id
	return ""


static func _tuning() -> BattleTuning:
	return BattleTuning.current()
