class_name WalkModes
extends RefCounted
## The movement modes as units walk (spec 30 round 3, part 6). A unit swims and climbs as
## its own walker allows (TerrainWalker.of_unit) at the pace the ground gives
## (FormationTerrain.factor), and:
## - a climb takes time: stepping into a cell more than a cliff above where it last stood,
##   it is on the face for that height, climbing at its climb pace before it moves on
##   ("climb 1 speed 2 could move 2 up a difficulty 1 surface in 1 movement");
## - it spends stamina for each second climbing or swimming (ModeStamina), so a slower
##   climb costs more;
## - it can't start a climb or a swim spent (it carries on one it is in);
## - it recovers none on a face or in deep water (FormationStamina reads ON_FACE);
## - spent on a face, it falls to the last place it stood, hurt by the height it had
##   climbed - fall_damage_per_cell a cell, times its footprint area to fall_mass_power.
## Pure over the unit, its columns in the store (`columns`: where it stands, its stamina,
## FOOTHOLD, ON_FACE and CLIMB_LEFT; spec 35 step 7c) and the ground.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const ModeStamina = preload("res://sim/skirmish/formation/mode_stamina.gd")
const ClimbStamina = preload("res://sim/skirmish/formation/climb_stamina.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001


## Whether `unit` at `from` may step towards `to`: walking always; into a climb or a swim
## unless it is spent and not already in that mode. `columns`: its store's, its STAMINA read.
static func may_step(
	unit, columns: UnitColumns, from: Vector2, to: Vector2, terrain: FormationTerrain
) -> bool:
	if terrain == null:
		return true
	var walker := TerrainWalker.of_unit(unit)
	var mode := terrain.mode(walker, from, to)
	if mode == TerrainWalker.Mode.WALKING or terrain.mode(walker, from, from) == mode:
		return true
	return ModeStamina.can_start(unit, columns.value(unit.slot, UnitColumns.STAMINA), mode)


## A tick of `seconds` on a climb face for `unit` of `squad` (anything holding the store),
## `cells_per_second` its pace on open ground: true while it is climbing (it moves no
## further this tick). Returns false once it is up, or if it has fallen.
static func climb(
	unit, squad, terrain: FormationTerrain, seconds: float, cells_per_second: float
) -> bool:
	if terrain == null:
		return false
	var columns: UnitColumns = squad.store.columns
	var left: float = columns.value(unit.slot, UnitColumns.CLIMB_LEFT)
	if left <= EPSILON:
		return false
	var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
	var share := terrain.factor(unit, columns.value(unit.slot, UnitColumns.FOOTHOLD), at)
	left -= share * cells_per_second * seconds
	columns.put(unit.slot, UnitColumns.CLIMB_LEFT, left)
	var stamina: float = columns.value(unit.slot, UnitColumns.STAMINA)
	stamina = maxf(0.0, stamina - ModeStamina.rate(TerrainWalker.Mode.CLIMBING) * seconds)
	columns.put(unit.slot, UnitColumns.STAMINA, stamina)
	var on_face := left > EPSILON
	columns.put(unit.slot, UnitColumns.ON_FACE, 1 if on_face else 0)
	if on_face and stamina <= 0.0:
		_fall(unit, squad, terrain)
		return false
	return on_face


## After `unit` stepped from `from` for `seconds`: begins a climb it stepped onto, charges
## a swim's stamina, and notes where it can stand.
static func after_step(
	unit, columns: UnitColumns, from: Vector2, terrain: FormationTerrain, seconds: float
) -> void:
	if terrain == null:
		return
	var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
	var foothold: Vector2 = columns.value(unit.slot, UnitColumns.FOOTHOLD)
	var walker := TerrainWalker.of_unit(unit)
	var mode := terrain.mode(walker, foothold, at)
	if mode == TerrainWalker.Mode.CLIMBING:
		columns.put(unit.slot, UnitColumns.CLIMB_LEFT, ClimbStamina.drop(terrain, foothold, at))
		columns.put(unit.slot, UnitColumns.ON_FACE, 1)
		return
	var here := terrain.mode(walker, at, at)
	if here == TerrainWalker.Mode.SWIMMING and from.distance_to(at) > EPSILON:
		var stamina: float = columns.value(unit.slot, UnitColumns.STAMINA)
		stamina = maxf(0.0, stamina - ModeStamina.rate(here) * seconds)
		columns.put(unit.slot, UnitColumns.STAMINA, stamina)
	columns.put(unit.slot, UnitColumns.ON_FACE, 1 if here == TerrainWalker.Mode.SWIMMING else 0)
	if ClimbStamina.standable(terrain, walker, at):
		columns.put(unit.slot, UnitColumns.FOOTHOLD, at)


## Drops `unit` from its climb face to where it last stood, hurt by the height it climbed.
static func _fall(unit, squad, terrain: FormationTerrain) -> void:
	var columns: UnitColumns = squad.store.columns
	var tuning := BattleTuning.current()
	var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)
	var foothold: Vector2 = columns.value(unit.slot, UnitColumns.FOOTHOLD)
	var left: float = columns.value(unit.slot, UnitColumns.CLIMB_LEFT)
	var climbed := maxf(0.0, ClimbStamina.drop(terrain, foothold, at) - left)
	var area := float(unit.footprint_width * unit.footprint_depth)
	var hurt := roundi(
		tuning.fall_damage_per_cell * climbed * DetMath.pow(area, tuning.fall_mass_power)
	)
	UnitHealth.hurt_one(squad, unit, hurt)
	# placement: a fall puts it back at the foot of its climb (Decision 139)
	columns.put(unit.slot, UnitColumns.POSITION, foothold)
	columns.put(unit.slot, UnitColumns.CLIMB_LEFT, 0.0)
	columns.put(unit.slot, UnitColumns.ON_FACE, 0)
