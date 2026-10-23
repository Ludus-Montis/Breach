class_name ModeStamina
extends RefCounted
## Stamina in the movement modes (spec 30): climbing and swimming cost stamina for each
## second spent in them (stamina_climb, stamina_swim), not for each cell - a unit at half
## the mode's pace takes twice as long over a step and pays twice. A spent unit
## (FormationStamina's last stage) can't start a climb or a swim; one already in the mode
## carries on. The march charges it and keeps the spent from starting one in the crate
## (native/rust/src/march/modes.rs, spec 39 step 3d). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")


## The stamina `walker`, at `speed` cells a second on open ground, spends stepping from
## `from` into `to`: the seconds the step takes at its pace there times its mode's rate.
## INF where it can't go. The crate's (native/rust/src/paths/climb.rs, spec 39 step 3c).
static func step_cost(
	terrain: FormationTerrain, walker: TerrainWalker, from: Vector2, to: Vector2, speed: float
) -> float:
	var tuning := BattleTuning.current()
	var rates := PackedFloat64Array([tuning.stamina_climb, tuning.stamina_swim])
	return CratePaths.core().step_cost(terrain.core, walker.fields(), from, to, speed, rates)
