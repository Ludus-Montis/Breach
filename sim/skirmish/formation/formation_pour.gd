class_name FormationPour
extends RefCounted
## Pouring through a gap (spec 30 round 3, part 2): a unit whose place lies on ground it
## can't walk makes for the nearest point across its frame it can (FormationWalk.target_of),
## and where several such points fall on one another, or on a place a unit already stands
## for, they file in behind it: each squeezed unit, front first, then nearest its frame's
## centre line, then by its seeded draw (Decision 97), takes the first point back from its
## own - a body's breadth at a time, against its formation's way - that it can walk and that
## no target taken before holds. So a formation queues at a ford and pours through it
## rather than packing its units onto one point. Pure; cells. A view over the crate's
## (native/rust/src/march/pour.rs, spec 39 step 3d).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## Where `unit` belongs in its squad's frame now (`basis`: StoredSquad.frame), standing at `place`
## (rank, column), including any swap under way (`moving`: FormationShuffle.movers for the
## squad, or null).
static func place(
	squad: SkirmishSquad, basis: Array, unit: SkirmishUnit, moving, place: Vector2i
) -> Vector2:
	var swapping := FormationShuffle.offset(squad, unit, moving)
	var anchor: Vector2 = basis[0]
	return CrateMarch.core().place(
		PackedFloat64Array([anchor.x, anchor.y, basis[1], basis[2], basis[3]]),
		PackedFloat64Array(
			[place.x, place.y, unit.footprint_width, unit.footprint_depth, swapping.x, swapping.y]
		)
	)


## unit id -> where each of the squad's units in its frame (and its loose ones, if
## `with_loose`) makes for: one out taking a
## downed foe, beside it; another, its place, or - where its place lies on ground it can't
## walk - the nearest it can walk, squeezing across the frame towards its centre line (so a
## formation pours through a gap narrower than itself and fans out again past it, and wades
## a ford rather than swim beside it: spec 30 round 3, parts 5 and 6), the squeezed filing
## in behind one another; with none to walk, its place if it can swim or climb there, else
## the nearest it can stand on.
static func targets(
	squad: SkirmishSquad,
	terrain: FormationTerrain = null,
	moving = null,
	fight_seed: int = 0,
	with_loose := false
) -> Dictionary:
	var units := []
	for unit in squad.living():
		if squad.fleeing.has(unit.id) or (squad.loose.has(unit.id) and not with_loose):
			continue
		units.append(unit)
	var out := {}
	if units.is_empty():
		return out
	var aims: PackedVector2Array = CrateMarch.core().targets(
		UnitMover.ground(terrain),
		CrateMarch.frame(squad),
		CrateMarch.pourers(squad, units, moving),
		fight_seed,
		BattleTuning.current().wounds_reach
	)
	for index in units.size():
		out[units[index].id] = aims[index]
	return out
