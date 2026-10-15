class_name RoutFlight
extends RefCounted
## How a disorderly flight spreads (Decision 99, spec 27 round 10): a fleeing unit makes for
## the nearest safety. With a standing friendly formation between it and home (its
## refuge), it steers for that friend, which catches it (RoutCatch). Otherwise a rout's
## units don't keep to their route's line but fan out from it, each by its own angle -
## seeded by the battle and the unit - up to rout_fan_degrees either side, until
## rout_fan_cells out (BattleTuning) or the ground to the side can't be crossed. A ragged
## retreat fans out by its disorder (FormationWithdraw). Its steps pay the ground they
## cross (UnitMover, Decision 139): a wood or a ford slows a flight as it slows a march.
## Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const RoutFriends = preload("res://sim/skirmish/formation/rout_friends.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


## The unit's own flight angle (radians) off its route's line, at full disorder.
static func fan(unit: SkirmishUnit, fight_seed: int) -> float:
	var roll := BattleRolls.uniform(fight_seed, [unit.id, "fan"]) * 2.0 - 1.0
	return deg_to_rad(BattleTuning.current().rout_fan_degrees) * roll


## A router's step (it has an entry in its squad's `fleeing`): towards `home` along the route
## and out to the side, or towards its refuge. `motion` is [pace (cells a tick at speed 1),
## fight seed, terrain or null, where it stands, the tick's RoutFriends].
static func step(squad: SkirmishSquad, unit: SkirmishUnit, home: float, motion: Array) -> void:
	var columns := squad.store.columns
	var full: float = unit.speed * motion[0]
	var aside := DetMath.sin(fan(unit, motion[1])) * full
	var fanned: float = columns.value(unit.slot, UnitColumns.FANNED)
	var along: float = columns.value(unit.slot, UnitColumns.ALONG)
	var heading: Vector2 = squad.route.heading_at(along) if squad.route != null else Vector2.RIGHT
	var side := heading.orthogonal()
	var at: Vector2 = motion[3]
	var refuge = _refuge(squad, at, [along, home, motion[1]], motion[4])
	if refuge != null:  # it makes for the friend, turning aside only if it would miss it
		var most := DetMath.sin(deg_to_rad(BattleTuning.current().rout_fan_degrees)) * full
		aside = clampf(RoutFriends.short_of(motion[4], refuge, at, side), -most, most)
	var terrain = motion[2]
	var open: bool = terrain == null or terrain.factor(unit, at, at + side * aside) > 0.0
	if (
		(refuge == null and absf(fanned + aside) > BattleTuning.current().rout_fan_cells)
		or not open
	):
		aside = 0.0
	var ahead := sqrt(full * full - aside * aside) * signf(home - along)
	var paid := UnitMover.share(unit, at, at + side * aside + heading * ahead, terrain)
	EntryKeys.keep(columns, unit.slot, UnitColumns.FANNED, fanned + aside * paid)
	var offset: Vector2 = columns.value(unit.slot, UnitColumns.FLEE_OFFSET) + side * aside * paid
	columns.put(unit.slot, UnitColumns.FLEE_OFFSET, offset)  # paid: UnitMover.share
	along = move_toward(along, home, absf(ahead) * paid)
	columns.put(unit.slot, UnitColumns.ALONG, along)  # paid: UnitMover.share


## The standing friendly formation whose unit is nearest `at`, among those between the
## router and home on the route - `along` is [its distance along it, home's, the fight
## seed] - or null if there is none. Equally near ones go by their draws (Decision 97).
static func _refuge(squad: SkirmishSquad, at: Vector2, along: Array, friends: Dictionary):
	if squad.route == null or friends.is_empty():
		return null  # no route, or no friend standing (FormationRout passes {})
	return RoutFriends.refuge(friends, squad, at, along)
