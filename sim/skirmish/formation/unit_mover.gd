class_name UnitMover
extends RefCounted
## The one way a unit moves (Decision 139): every step a unit takes - walking to its place,
## loose in a fight, chasing, regrouping, withdrawing, fleeing - goes from where it stands
## towards where it means to go, at its pace by facing (UnitMotion), and pays the ground it
## crosses (TerrainStep.pay: cost, slope, liquid, barriers, its climbing and swimming).
## Behaviours decide where a unit goes and how far it may go this tick; only this moves it.
## A position written anywhere else is a placement, not a move (a carried body on its
## bearer, a frame's slot, bodies parted) and says so - check_one_mover.py holds the line.
## Pure.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## Where the unit at `at` gets walking towards `to` with `full` cells at full pace this
## tick, turning as it goes (towards its heading, or to keep facing `look`) over `seconds`,
## on `terrain` (null: open). Its bearing is in the store's `columns`.
static func step(
	unit: SkirmishUnit,
	columns: UnitColumns,
	at: Vector2,
	to: Vector2,
	full: float,
	seconds: float,
	terrain: FormationTerrain,
	look = null
) -> Vector2:
	return pay(unit, at, UnitMotion.walk(unit, columns, at, to, full, seconds, look), terrain)


## As step, for a unit that doesn't turn as it moves (it turns in its own pass).
static func slide(
	unit: SkirmishUnit,
	columns: UnitColumns,
	at: Vector2,
	to: Vector2,
	full: float,
	terrain: FormationTerrain
) -> Vector2:
	return pay(unit, at, UnitMotion.move(unit, columns, at, to, full), terrain)


## Where a step from `at` to `stepped` worked out on open ground (the native walk's, or
## step's own) lands once the ground is paid.
static func pay(
	unit: SkirmishUnit, at: Vector2, stepped: Vector2, terrain: FormationTerrain
) -> Vector2:
	return TerrainStep.pay(unit, at, stepped, terrain)


## The share of an open-ground step from `at` to `stepped` the ground lets the unit make:
## 1 on open ground, 0 where it's barred. For movers that keep their own coordinates (a
## router's distance along its route and offset from it).
static func share(
	unit: SkirmishUnit, at: Vector2, stepped: Vector2, terrain: FormationTerrain
) -> float:
	var wanted := at.distance_to(stepped)
	if terrain == null or wanted < 0.000001:
		return 1.0
	return at.distance_to(pay(unit, at, stepped, terrain)) / wanted
