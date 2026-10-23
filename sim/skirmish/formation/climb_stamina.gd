class_name ClimbStamina
extends RefCounted
## Climbing on stamina (spec 30). There is no breather on a climb face: stamina recovers
## only where a unit can stand - a cell it can walk (not swim) whose slope across it is no
## more than the cliff threshold (ground_cliff_quarters). A cell's slope is the steepest
## steady climb through it along either axis: how far it rises from one side's neighbour and
## on to the other's, the lesser of the two. A cliff's foot and top are flat (one side is
## level), a 1-cell step in a staircase of cliffs is a face, a 2-cell ledge is not; a pit or
## a peak is flat. A stretch of a climb runs between standable cells; a pathfinder plans
## only stretches its unit's stamina lasts (the crate's climb budget,
## native/rust/src/paths/climb.rs). A unit at 0 stamina on a face falls to the foot of its
## stretch, the last standable cell it left (the march knows it), hurt fall_damage_per_cell
## for each cell of the drop times its mass (its footprint, as bodies weigh it, Decision
## 121) to the power fall_mass_power. Recovery and falls are for the march to apply (spec 30
## part 6). Noted for later, not built: a `cat_fall` trait taking less fall damage, and a
## feather-fall ability taking none. Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const ModeStamina = preload("res://sim/skirmish/formation/mode_stamina.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

## Quarter-cells in a cell of height.
const QUARTERS := 4.0


## Whether `walker` can stand (and so recover stamina) on the cell holding `at`: the
## crate's rule (native/rust/src/paths/climb.rs, spec 39 step 3c).
static func standable(terrain: FormationTerrain, walker: TerrainWalker, at: Vector2) -> bool:
	var cliff := BattleTuning.current().ground_cliff_quarters
	return CratePaths.core().standable(terrain.core, walker.fields(), at, cliff)


## Whether a unit with `stamina` left at `at` falls: spent, and not where it can stand.
static func falls(
	stamina: float, terrain: FormationTerrain, walker: TerrainWalker, at: Vector2
) -> bool:
	return stamina <= 0.0 and not standable(terrain, walker, at)


## How far (cells) a unit at `at` falls to the foot `foot` of its stretch.
static func drop(terrain: FormationTerrain, foot: Vector2, at: Vector2) -> float:
	return maxf(0.0, terrain.height_at(at) - terrain.height_at(foot)) / QUARTERS


## The hurt a fall of `cells` does a unit of `unit_def`.
static func fall_damage(cells: float, unit_def: UnitDef) -> float:
	var tuning := BattleTuning.current()
	var mass := float(unit_def.footprint_width * unit_def.footprint_depth)
	return tuning.fall_damage_per_cell * cells * DetMath.pow(mass, tuning.fall_mass_power)


## The most stamina any stretch between standable cells of the way `cells` costs `walker`.
static func hardest(terrain: FormationTerrain, walker: TerrainWalker, cells: Array) -> float:
	var worst := 0.0
	var stretch := 0.0
	for index in range(1, cells.size()):
		var from := Vector2(cells[index - 1]) + Vector2(0.5, 0.5)
		var to := Vector2(cells[index]) + Vector2(0.5, 0.5)
		stretch += ModeStamina.step_cost(terrain, walker, from, to, walker.speed)
		worst = maxf(worst, stretch)
		if standable(terrain, walker, to):
			stretch = 0.0
	return worst
