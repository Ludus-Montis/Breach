class_name TerrainWalker
extends RefCounted
## How a unit crosses terrain (spec 30, Decisions 64 and 85): its height, which sets the
## bands of liquid it wades, and its movement modes. Climbing and swimming follow one
## rule: the mode's base pace (BattleTuning) times Decision 64's pair rule, its ability
## (`climber N`, `swimmer N`; every unit has both at 0) against the ground's demand (a
## cliff's climb difficulty, water's `flows`). A unit loaded past the threshold uses
## neither mode; one that "sinks" never swims and one that "cant_climb" never climbs.
## FormationTerrain reads it for the pace of each step, so the march and paths can't
## disagree. Pure.

## The movement mode of a step (FormationTerrain.mode): stamina is paid per second in
## the climbing and swimming modes (ModeStamina).
enum Mode { WALKING, CLIMBING, SWIMMING }

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")

## How tall it stands, in cells.
var height: float
## Its abilities in the two modes, met against the ground's demands.
var swimmer: int
var climber: int
## Whether it may swim or climb at all.
var swims: bool
var climbs: bool
## The stamina it has for a climb (INF: no limit) and its speed on open ground (cells a
## second): a path plans only climbs whose stretches between ledges it lasts (ClimbStamina).
## Not part of its kind: they don't change its pace.
var stamina := INF
var speed := 1.0


## How a unit of `unit_def` crosses terrain: its abilities (an unlisted one at its
## implicit level), and the modes its traits and load allow it.
static func of(unit_def: UnitDef) -> TerrainWalker:
	var burdened := _too_heavy_for_modes(unit_def)
	return TerrainWalker.new(
		unit_def.height,
		unit_def.trait_level("swimmer"),
		unit_def.trait_level("climber"),
		not burdened and unit_def.trait_level("sinks") == 0,
		not burdened and unit_def.trait_level("cant_climb") == 0
	)


## The walker a unit crosses terrain as: its own (FormationUnits gives each one from its
## definition), or - for a unit made without one - a grounded one of its height.
static func of_unit(unit) -> TerrainWalker:
	if unit.walker != null:
		return unit.walker
	return grounded(unit.height)


## What of it the crate's crossing reads: [height, swimmer, climber, swims, climbs].
func fields() -> PackedFloat64Array:
	return PackedFloat64Array([height, swimmer, climber, float(swims), float(climbs)])


## A unit `unit_height` cells tall that neither swims nor climbs.
static func grounded(unit_height: float) -> TerrainWalker:
	return TerrainWalker.new(unit_height, 0, 0, false, false)


## A key naming what decides its pace on every cell: walkers with the same key share
## their speed shares (WalkerShares). The height goes in by its exact bits, never as text:
## how a float is written differs between platforms (Decision 130).
func kind() -> String:
	var bits := PackedFloat64Array([height]).to_byte_array().hex_encode()
	return "%s|%d|%d|%d|%d" % [bits, swimmer, climber, int(swims), int(climbs)]


## Decision 64's pair rule, an ability against a demand: the share of the mode's pace it
## goes at - 1 meeting it, half one level short, 0 further short.
static func meets(ability: int, demand: int) -> float:
	if ability >= demand:
		return 1.0
	return 0.5 if ability == demand - 1 else 0.0


## Whether it carries more than ground_mode_load_share of the way through its first
## encumbrance band (from load_easy of its carry limit to the limit).
static func _too_heavy_for_modes(unit_def: UnitDef) -> bool:
	var tuning := BattleTuning.current()
	var limit := unit_def.strength * tuning.load_per_strength
	var easy := limit * tuning.load_easy
	return UnitArms.carried_weight(unit_def) > easy + (limit - easy) * tuning.ground_mode_load_share


func _init(
	unit_height: float,
	swimmer_level: int = 0,
	climber_level: int = 0,
	can_swim: bool = true,
	can_climb: bool = true
) -> void:
	height = unit_height
	swimmer = swimmer_level
	climber = climber_level
	swims = can_swim
	climbs = can_climb
