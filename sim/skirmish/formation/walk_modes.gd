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
## - it recovers none on a face or in deep water (FormationStamina reads `on_face`);
## - spent on a face, it falls to the last place it stood, hurt by the height it had
##   climbed - fall_damage_per_cell a cell, times its footprint area to fall_mass_power.
## Pure over the unit and the ground.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const ModeStamina = preload("res://sim/skirmish/formation/mode_stamina.gd")
const ClimbStamina = preload("res://sim/skirmish/formation/climb_stamina.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

const EPSILON := 0.000001


## Whether `unit` at `from` may step towards `to`: walking always; into a climb or a swim
## unless it is spent and not already in that mode.
static func may_step(unit, from: Vector2, to: Vector2, terrain: FormationTerrain) -> bool:
	if terrain == null:
		return true
	var walker := TerrainWalker.of_unit(unit)
	var mode := terrain.mode(walker, from, to)
	if mode == TerrainWalker.Mode.WALKING or terrain.mode(walker, from, from) == mode:
		return true
	return ModeStamina.can_start(unit, mode)


## A tick of `seconds` on a climb face for `unit`, `cells_per_second` its pace on open
## ground: true while it is climbing (it moves no further this tick). Returns false once
## it is up, or if it has fallen.
static func climb(unit, terrain: FormationTerrain, seconds: float, cells_per_second: float) -> bool:
	if terrain == null or unit.climb_left <= EPSILON:
		return false
	var share := terrain.factor(unit, unit.foothold, unit.position)
	unit.climb_left -= share * cells_per_second * seconds
	unit.stamina = maxf(0.0, unit.stamina - ModeStamina.rate(TerrainWalker.Mode.CLIMBING) * seconds)
	unit.on_face = unit.climb_left > EPSILON
	if unit.on_face and unit.stamina <= 0.0:
		_fall(unit, terrain)
		return false
	return unit.on_face


## After `unit` stepped from `from` for `seconds`: begins a climb it stepped onto, charges
## a swim's stamina, and notes where it can stand.
static func after_step(unit, from: Vector2, terrain: FormationTerrain, seconds: float) -> void:
	if terrain == null:
		return
	var walker := TerrainWalker.of_unit(unit)
	var mode := terrain.mode(walker, unit.foothold, unit.position)
	if mode == TerrainWalker.Mode.CLIMBING:
		unit.climb_left = ClimbStamina.drop(terrain, unit.foothold, unit.position)
		unit.on_face = true
		return
	var here := terrain.mode(walker, unit.position, unit.position)
	if here == TerrainWalker.Mode.SWIMMING and from.distance_to(unit.position) > EPSILON:
		unit.stamina = maxf(0.0, unit.stamina - ModeStamina.rate(here) * seconds)
	unit.on_face = here == TerrainWalker.Mode.SWIMMING
	if ClimbStamina.standable(terrain, walker, unit.position):
		unit.foothold = unit.position


## Drops `unit` from its climb face to where it last stood, hurt by the height it climbed.
static func _fall(unit, terrain: FormationTerrain) -> void:
	var tuning := BattleTuning.current()
	var climbed := maxf(
		0.0, ClimbStamina.drop(terrain, unit.foothold, unit.position) - unit.climb_left
	)
	var area := float(unit.footprint_width * unit.footprint_depth)
	unit.hp -= roundi(
		tuning.fall_damage_per_cell * climbed * DetMath.pow(area, tuning.fall_mass_power)
	)
	# placement: a fall puts it back at the foot of its climb (Decision 139)
	unit.position = unit.foothold
	unit.climb_left = 0.0
	unit.on_face = false
