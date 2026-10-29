class_name TerrainStep
extends RefCounted
## The ground's toll on a unit's step (Decision 85; spec 30 round 3): one rule for every
## unit that walks - in its place (FormationWalk), loose in a fight (FormationScrum),
## regrouping (ScrumRegroup) or chasing (ScrumPursuit). A step goes at the share of its
## pace the ground it crosses allows (terrain.factor: a wood at half pace, a ford at its
## wading pace); where its way is barred (deep water, a wall) it slides along the barrier on
## whichever axis is open and brings it nearer (a stream's bank, a wall's face), and where
## neither is, it stays. A view over the crate's (native/rust/src/mover.rs, spec 39 step
## 3b); where loose units walk back to on the ground is the crate's own (the pour's targets,
## `scrum::regroup`, spec 39 step 8b5-xiv). Cells.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


## [the point the unit at `at` steps towards on its way to `goal`, the share of its pace
## the ground allows]: straight there; or, where that way is barred, along the barrier on
## whichever axis is open and brings it nearer; [at, 0] if none is. It looks `reach` cells
## ahead (at least where its step lands).
static func way(
	unit: SkirmishUnit, at: Vector2, goal: Vector2, terrain: FormationTerrain, reach := 1.0
) -> Array:
	var going: PackedFloat64Array = CrateMoves.core().way(
		UnitMover.ground(terrain), UnitMover.walker(unit, terrain), at, goal, reach
	)
	return [Vector2(going[0], going[1]), going[2]]


## Where the unit stepping from `at` to `to` - its step on open ground - gets on `terrain`
## (null: open): as far along its way (way) as the ground's share of that step takes it.
static func pay(unit: SkirmishUnit, at: Vector2, to: Vector2, terrain: FormationTerrain) -> Vector2:
	if terrain == null:
		return to
	return CrateMoves.core().pay(terrain.core, UnitMover.walker(unit, terrain), at, to)
