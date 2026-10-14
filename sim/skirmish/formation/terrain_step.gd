class_name TerrainStep
extends RefCounted
## The ground's toll on a unit's step (Decision 85; spec 30 round 3): one rule for every
## unit that walks - in its place (FormationWalk), loose in a fight (FormationScrum),
## regrouping (ScrumRegroup) or chasing (ScrumPursuit). A step goes at the share of its
## pace the ground it crosses allows (terrain.factor: a wood at half pace, a ford at its
## wading pace); where its way is barred (deep water, a wall) it slides along the barrier on
## whichever axis is open and brings it nearer (a stream's bank, a wall's face), and where
## neither is, it stays. Pure; cells.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationPour = preload("res://sim/skirmish/formation/formation_pour.gd")

const EPSILON := 0.000001


## [the point the unit at `at` steps towards on its way to `goal`, the share of its pace
## the ground allows]: straight there; or, where that way is barred, along the barrier on
## whichever axis is open and brings it nearer; [at, 0] if none is. It looks `reach` cells
## ahead (at least where its step lands).
static func way(
	unit: SkirmishUnit, at: Vector2, goal: Vector2, terrain: FormationTerrain, reach := 1.0
) -> Array:
	var to_goal := goal - at
	if terrain == null or to_goal.length() < EPSILON:
		return [goal, 1.0]
	var straight := terrain.factor(unit, at, at + to_goal.normalized() * reach)
	if straight > 0.0:
		return [goal, straight]
	var best := [at, 0.0]
	var nearest := to_goal.length()
	for along: Vector2 in [Vector2(to_goal.x, 0.0), Vector2(0.0, to_goal.y)]:
		if along.length() < EPSILON:
			continue
		var share := terrain.factor(unit, at, at + along.normalized() * reach)
		var left := (at + along).distance_to(goal)
		if share > 0.0 and left < nearest - EPSILON:
			best = [at + along, share]
			nearest = left
	return best


## Where the unit stepping from `at` to `to` - its step on open ground - gets on `terrain`
## (null: open): as far along its way (way) as the ground's share of that step takes it.
static func pay(unit: SkirmishUnit, at: Vector2, to: Vector2, terrain: FormationTerrain) -> Vector2:
	if terrain == null:
		return to
	var step := at.distance_to(to)
	var going := way(unit, at, to, terrain, maxf(step, 1.0))
	return at.move_toward(going[0], step * going[1])


## unit id -> where the squad's loose units walk back to on `terrain`: where each makes for
## in its frame (FormationPour.targets), so one whose place is barred makes for the nearest
## it can walk, as its formation's walk would - or {} (their places, ScrumStance.anchor) on
## open ground or in a stance.
static func returns(squad: SkirmishSquad, terrain: FormationTerrain, fight_seed: int) -> Dictionary:
	if terrain == null or not squad.stance.is_empty():
		return {}
	return FormationPour.targets(squad, terrain, null, fight_seed, true)
