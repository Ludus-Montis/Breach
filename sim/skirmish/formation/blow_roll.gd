class_name BlowRoll
extends RefCounted
## How a blow lands (Decision 118, spec 28 part 4): one seeded roll (Decision 93), the
## striker's skill plus a roll about it, its margin read against the target's layers in
## the order the user gave - its parry (melee only, with a weapon or shield), then its
## dodge (its agility), then its defence, which a blow short of grazes - then a hit, and a
## margin past the hit band a critical (the striker's critical multiplier). High ground
## and being surrounded add to the margin, and morale shifts it by band; a blow from the
## target's flank finds no parry, one from its rear no dodge either. These replace the
## flank, high ground and morale multipliers on damage; what a blow deals is BlowDamage's.
## All numbers: BattleTuning. Pure.

enum Side { FRONT, FLANK, REAR }

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

const PARRIED := "parried"
const DODGED := "dodged"
const GRAZED := "grazed"
const HIT := "hit"
const CRITICAL := "critical"


## How a blow with this margin lands on `target`, struck from `side` (at range: no parry).
static func outcome(margin: float, target: SkirmishUnit, side: int, ranged: bool) -> String:
	var tuning := BattleTuning.current()
	var parry := 0.0
	if target.parries and not ranged and side == Side.FRONT:
		parry = target.melee_skill * tuning.blow_parry_share
	var dodge := 0.0
	if side != Side.REAR:
		var agility: float = target.attributes.get("agility", UnitDef.AVERAGE)
		dodge = agility * tuning.blow_dodge_per_agility * target.dodging
	var layers := [
		[PARRIED, parry],
		[DODGED, dodge],
		[GRAZED, float(target.defence)],
		[HIT, tuning.blow_hit_band]
	]
	var reached := 0.0
	for layer in layers:
		reached += layer[1]
		if layer[1] > 0.0 and margin < reached:
			return layer[0]
	return CRITICAL


## The blow's margin: the striker's skill, plus `shift`, plus `roll` (0 to 1) across the
## die about it.
static func margin(striker: SkirmishUnit, ranged: bool, shift: float, roll: float) -> float:
	var skill := striker.ranged_skill if ranged else striker.melee_skill
	return skill + shift + (roll - 0.5) * BattleTuning.current().blow_die


## The side of the target standing at `at` facing `bearing` a blow from `from` falls on: its
## front unless `flank` (from outside its front); then its rear if within the rear arc of
## straight behind it.
static func side(bearing: float, at: Vector2, from: Vector2, flank: bool) -> int:
	if not flank:
		return Side.FRONT
	var away := from - at
	if away.length() < 0.000001:
		return Side.FLANK
	var behind := DetMath.cos(deg_to_rad(BattleTuning.current().blow_rear_arc_degrees))
	var facing := away.normalized().dot(UnitMotion.vector(bearing))
	return Side.REAR if facing <= -behind + 0.000001 else Side.FLANK


## Whether a shot from `from` comes from outside the front of the target standing at `at`
## facing `bearing`.
static func from_flank(bearing: float, at: Vector2, from: Vector2) -> bool:
	return not ScrumReach.in_front(bearing, at, from)
