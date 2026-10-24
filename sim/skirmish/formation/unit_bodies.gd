class_name UnitBodies
extends RefCounted
## Bodies (Decision 106, spec 30 round 1): a unit's body is the circle inscribed in its
## footprint - a grem 1 cell across, a brute 2 - and no two bodies overlap. After every
## move each tick, each pair whose bodies overlap is pushed apart the shortest way:
## - **Friends** split the push by mass (footprint area): a brute moves a grem more than a
##   grem moves a brute. A unit in its formation's frame resists, bodies_resist times its mass,
##   but gives way: pushed, it leaves its place and walks back to it (its squad re-forms).
##   To another squad's loose unit it doesn't give way - a squad's space is kept by its
##   units in control of it - so the loose one takes the whole push; nor to its own
##   squad's when only brushed (under bodies_brush deep). Routers in flight still push through,
##   and its own squad's loose units still shove their way through it.
## - **Foes** are not pushed (Decision 105): two that overlap - one shoved into the other by
##   its friends - each step back half the overlap, so no one is pushed into an enemy.
## - **Two units both in their frames:** of two squads, they are kept apart by the frames
##   (friends queueing between squads: Decision 84; foes halting at contact). Of one squad,
##   nothing else keeps them apart once they walk to their places (they can cross, or close
##   up in a queue at a gap), so they part as friends do, but each stays in its frame and
##   walks on from where it was pushed: only a push from a body out of its frame takes a
##   unit out of its own.
## Pushes are worked out from one snapshot, then applied together, bodies_passes times a tick; a
## pair whose bodies lie exactly on each other parts along an angle seeded by the battle
## and the two units, never by a world direction or the list (Decision 97). The passes are
## the crate's (`parting.rs`, on the battle's BodyField: BodyParting); spec 39 step 6a
## retired their GDScript twin. Pure over the squads it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const BodyParting = preload("res://sim/skirmish/formation/body_parting.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

## Microseconds spent in step() since it was last zeroed (the bench reads it; no outcome does).
static var clock_usec := 0


## The body's radius in cells.
static func radius(unit: SkirmishUnit) -> float:
	return ScrumReach.radius(unit)


## Where the unit's body stands: its point in the scrum or in flight, or its place.
static func at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	if squad.fleeing.has(unit.id):
		return FormationRout.where(squad, unit.id)
	if squad.loose.has(unit.id):
		return SquadEntries.loose_at(squad, unit)
	return squad.store.columns.value(unit.slot, UnitColumns.POSITION)


## Pushes apart every pair of bodies that overlap, on the battle's native `field` (a
## BodyField kept across ticks; without one, its squads' store's).
static func step(squads: Array, fight_seed: int, field: Object = null) -> void:
	var began := Time.get_ticks_usec()
	if field == null and not squads.is_empty():
		field = squads[0].store.field()
	if field != null:
		var way := func(draw: int, other_draw: int) -> Vector2:
			return part_way(fight_seed, draw, other_draw)
		BodyParting.step(field, squads, fight_seed, way, NativeKernels.threaded)
	clock_usec += Time.get_ticks_usec() - began


## The way two bodies lying exactly on each other part: seeded by the battle and their draws.
static func part_way(fight_seed: int, draw: int, other_draw: int) -> Vector2:
	var angle := TAU * BattleRolls.uniform(fight_seed, [draw, other_draw, "part"])
	return DetMath.rotated(Vector2.RIGHT, angle)
