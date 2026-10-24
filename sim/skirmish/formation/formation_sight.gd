class_name FormationSight
extends RefCounted
## What a formation can detect (Decision 87, spec 27 round 2): another squad is detected if
## any of its living units is within the best detection range among the formation's living
## units, measured in cells between unit centres. Nothing else is known to it - no formation
## is telepathic. With terrain, a sight line that passes through more than two cells that
## block sight (a wood) is blocked: units at a wood's edge see out and are seen, deeper in
## they are hidden (Decision 87). Light comes later. Pure. The rules are the crate's
## (`perception.rs`, spec 39 step 4c).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")


## True if `squad` detects `other` on `terrain` (null: open ground). `listed`: [a roster
## (CrateRoster) holding both, the squad's index in it, the other's], made here if empty.
static func detects(
	squad: SkirmishSquad, other: SkirmishSquad, terrain: FormationTerrain = null, listed := []
) -> bool:
	if listed.is_empty():
		listed = [CrateRoster.of([squad, other]), 0, 1]
	var field: Object = squad.store.field()
	return field.sight_detects(listed[0], listed[1], listed[2], UnitMover.ground(terrain))


## True if the sight line between two points crosses at most two blocking cells.
static func clear(terrain: FormationTerrain, from: Vector2, to: Vector2) -> bool:
	return terrain.core.screens_clear(from, to)
