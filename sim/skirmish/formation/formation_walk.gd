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
## squad sets its units on their places (FormationMarch.sync_units). Pure. A view over the
## crate's (native/rust/src/march/walk.rs and modes.rs, spec 39 step 3d): the walk, its
## climbing and swimming (modes.rs, once WalkModes), the slack and the rule; the share is
## the move pass's (native/rust/src/moving/share.rs, spec 39 step 8b5-ix).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationPour = preload("res://sim/skirmish/formation/formation_pour.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001
## The columns a walk reads and writes, held for it (UnitView): where each stands and last
## stood, its stamina, whether it is on a climb face or in deep water, the climb left, its
## bearing and its pace.
const WALKS := [
	UnitColumns.POSITION,
	UnitColumns.FOOTHOLD,
	UnitColumns.STAMINA,
	UnitColumns.ON_FACE,
	UnitColumns.CLIMB_LEFT,
	UnitColumns.BEARING,
	UnitColumns.SPEED,
]


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
	var held := UnitView.hold([squad], WALKS)
	walk_all(squad, held, [unit], {unit.id: place}, timing, terrain)
	UnitView.release(held)


## `walk` for each of `units` of `squad`, towards its point in `aims` (unit id -> point):
## one call for the squad. `held` is the squad's view of a hold of WALKS (the march's,
## FormationMarch.WALKED): each unit reads its own columns there, the ground and the bodies
## lying, so the order they walk in can't matter; a fall hurts it (UnitHealth) after, in
## their order.
static func walk_all(
	squad: SkirmishSquad,
	held: UnitView,
	units: Array,
	aims: Dictionary,
	timing: Array,
	terrain: FormationTerrain = null
) -> void:
	if units.is_empty():
		return
	var points: PackedVector2Array = held.position
	var footholds: PackedVector2Array = held.foothold
	var stamina: PackedFloat64Array = held.stamina
	var on_face: PackedInt32Array = held.on_face
	var climb_left: PackedFloat64Array = held.climb_left
	var bearings: PackedFloat64Array = held.bearing
	var speeds: PackedFloat64Array = held.speed
	var marchers := PackedFloat64Array()
	var states := PackedFloat64Array()
	var places := PackedVector2Array()
	for unit in units:
		var slot: int = unit.slot
		marchers.append_array(CrateMarch.marcher(unit))
		var at := [points[slot].x, points[slot].y, footholds[slot].x, footholds[slot].y]
		var rest := [stamina[slot], on_face[slot], climb_left[slot], bearings[slot], speeds[slot]]
		states.append_array(PackedFloat64Array(at + rest))
		places.append(aims[unit.id])
	var bodies := timing.size() > 2
	var lying: Array = [PackedVector2Array(), PackedFloat64Array()]
	if bodies:
		lying = CrateMarch.lying(timing[2])
	var walked: PackedFloat64Array = CrateMarch.core().walk(
		UnitMover.ground(terrain),
		bodies,
		lying[0],
		lying[1],
		PackedFloat64Array([timing[0], timing[1], squad.heading]),
		marchers,
		states,
		places,
		CrateMarch.walk_tuning()
	)
	var fell := []
	for index in units.size():
		var slot: int = units[index].slot
		var w := index * 10
		points[slot] = Vector2(walked[w], walked[w + 1])  # stepped by the crate's one mover
		footholds[slot] = Vector2(walked[w + 2], walked[w + 3])
		stamina[slot] = walked[w + 4]
		on_face[slot] = int(walked[w + 5])
		climb_left[slot] = walked[w + 6]
		bearings[slot] = walked[w + 7]
		if walked[w + 9] != 0.0:  # it fell from a climb face (march/modes.rs)
			fell.append([units[index], int(walked[w + 9])])
	# store: scattered (few: the units that fell from a climb face this tick)
	for falling in fell:
		UnitHealth.hurt_one(squad, falling[0], falling[1])


## Where `unit` belongs in its squad's frame now, including any swap under way (`moving`:
## FormationShuffle.movers for the squad, or null; `places`: a view of the squad's store
## holding UnitView.PLACES).
static func place_of(
	squad: SkirmishSquad, basis: Array, unit: SkirmishUnit, moving, places: UnitView
) -> Vector2:
	return FormationPour.place(squad, basis, unit, moving, UnitPlace.in_view(places, unit))


## `place_of` for one unit, its place read from the store: scattered, for an event's few.
static func place_of_one(squad: SkirmishSquad, unit: SkirmishUnit, moving = null) -> Vector2:
	return FormationPour.place(squad, squad.frame(), unit, moving, UnitPlace.of(squad, unit))


## The share (0 to 1) of its step the squad's frame takes this tick for its units lagging
## behind their places: all of it while every unit is within half its slack, easing to
## none as the furthest nears the slack itself, so a wheel slows to what its outer file
## can walk rather than halting; one fallen behind (walk_lost) is waited for only if no
## man is left behind. The crate's (`moving::share`, spec 39 step 8b5-ix), each unit read
## where it lives.
static func share(
	squad: SkirmishSquad, terrain: FormationTerrain = null, fight_seed: int = 0
) -> float:
	var tuning := _tuning()
	var rules := PackedFloat64Array(
		[
			tuning.discipline_per_leadership,
			tuning.walk_slack_loose,
			tuning.walk_slack_drilled,
			tuning.wounds_reach,
			tuning.walk_lost,
		]
	)
	var roster := CrateRoster.of([squad])
	return squad.store.field().walk_share(roster, UnitMover.ground(terrain), fight_seed, rules)


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
	return CrateMarch.core().slack_of(
		FormationDiscipline.of(squad), tuning.walk_slack_loose, tuning.walk_slack_drilled
	)


## "fall_behind_left_behind", "no_man_left_behind" or "": a leader's trait, or one every
## unit has (the never-wait wins if both are found).
static func rule_of(squad: SkirmishSquad) -> String:
	return CrateMarch.RULES[CrateMarch.core().rule_of(_traits(squad.living()))]


static func _tuning() -> BattleTuning:
	return BattleTuning.current()


## The living units' [leads, has fall_behind_left_behind, has no_man_left_behind].
static func _traits(living: Array) -> PackedInt32Array:
	var out := PackedInt32Array()
	for unit in living:
		out.append(int(unit.leadership > 0))
		out.append(int(unit.traits.has(CrateMarch.RULES[1])))
		out.append(int(unit.traits.has(CrateMarch.RULES[2])))
	return out
