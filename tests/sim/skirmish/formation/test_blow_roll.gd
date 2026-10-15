extends GdUnitTestSuite
## BlowRoll, per Decision 118 (spec 28 part 4): a blow's margin - skill plus a roll about
## it - read against the target's parry, dodge and defence in that order, then a hit, past
## the hit band a critical; a flank finds no parry, the rear no dodge either.

const BlowRoll = preload("res://sim/skirmish/formation/blow_roll.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")

const EAST := 90.0  # the bearing of a target facing east


## A target: skill 40, agility 10 and defence 10 - with the usual tuning a parry of 10, a
## dodge of 10, grazes to 30 and hits to 90.
func _target(parries: bool) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.melee_skill = 40
	unit.attributes = {"agility": 10}
	unit.defence = 10
	unit.parries = parries
	return unit


func test_the_margin_falls_through_parry_dodge_defence_and_hit_into_a_critical() -> void:
	var target := _target(true)
	var front := BlowRoll.Side.FRONT

	assert_str(BlowRoll.outcome(-20.0, target, front, false)).is_equal(BlowRoll.PARRIED)
	assert_str(BlowRoll.outcome(5.0, target, front, false)).is_equal(BlowRoll.PARRIED)
	assert_str(BlowRoll.outcome(15.0, target, front, false)).is_equal(BlowRoll.DODGED)
	assert_str(BlowRoll.outcome(25.0, target, front, false)).is_equal(BlowRoll.GRAZED)
	assert_str(BlowRoll.outcome(50.0, target, front, false)).is_equal(BlowRoll.HIT)
	assert_str(BlowRoll.outcome(95.0, target, front, false)).is_equal(BlowRoll.CRITICAL)


func test_a_flank_finds_no_parry_the_rear_no_dodge_and_range_no_parry() -> void:
	var target := _target(true)

	assert_str(BlowRoll.outcome(5.0, target, BlowRoll.Side.FLANK, false)).is_equal(BlowRoll.DODGED)
	assert_str(BlowRoll.outcome(5.0, target, BlowRoll.Side.REAR, false)).is_equal(BlowRoll.GRAZED)
	assert_str(BlowRoll.outcome(-40.0, target, BlowRoll.Side.REAR, false)).is_equal(BlowRoll.GRAZED)
	assert_str(BlowRoll.outcome(5.0, target, BlowRoll.Side.FRONT, true)).is_equal(BlowRoll.DODGED)
	assert_str(BlowRoll.outcome(5.0, _target(false), BlowRoll.Side.FRONT, false)).is_equal(
		BlowRoll.DODGED
	)


func test_the_margin_is_skill_and_shift_with_the_roll_across_the_die() -> void:
	var striker := SkirmishUnit.new()
	striker.melee_skill = 40
	striker.ranged_skill = 70
	var die := BattleTuning.current().blow_die

	assert_float(BlowRoll.margin(striker, false, 15.0, 0.5)).is_equal_approx(55.0, 0.0001)
	assert_float(BlowRoll.margin(striker, false, 0.0, 0.0)).is_equal_approx(
		40.0 - die / 2.0, 0.0001
	)
	assert_float(BlowRoll.margin(striker, true, 0.0, 0.5)).is_equal_approx(70.0, 0.0001)


func test_the_side_a_blow_falls_on() -> void:
	var at := Vector2.ZERO  # a target at the origin, facing east

	assert_int(BlowRoll.side(EAST, at, Vector2(-1, 0), false)).is_equal(BlowRoll.Side.FRONT)
	assert_int(BlowRoll.side(EAST, at, Vector2(0, 1), true)).is_equal(BlowRoll.Side.FLANK)
	assert_int(BlowRoll.side(EAST, at, Vector2(-1, 0.2), true)).is_equal(BlowRoll.Side.REAR)
	assert_bool(BlowRoll.from_flank(EAST, at, Vector2(3, 0))).is_false()
	assert_bool(BlowRoll.from_flank(EAST, at, Vector2(-3, 0))).is_true()


func test_a_unit_parries_with_a_held_weapon_or_a_shield_not_its_claws() -> void:
	var claws := UnitDef.new()
	claws.items = [WeaponDef.innate_weapon(3, "claw")]
	var shield := ItemDef.new()
	shield.item_name = "shield"
	shield.tags = ["shield"]
	var shielded := UnitDef.new()
	shielded.items = [WeaponDef.innate_weapon(3, "claw"), shield]
	var militia: UnitDef = load("res://content/units/kingdom_militia.tres")

	assert_bool(claws.can_parry()).is_false()
	assert_bool(shielded.can_parry()).is_true()
	assert_bool(militia.can_parry()).is_true()


func test_a_unit_in_the_field_carries_its_skills() -> void:
	var captain: UnitDef = load("res://content/units/kingdom_captain.tres")
	var unit := FormationUnits.make(captain, "the_kingdom", -1, 1)

	assert_int(unit.melee_skill).is_equal(
		captain.melee_skill + roundi(BattleTuning.current().item_mastered_skill)
	)  # a mastered polearm
	assert_int(unit.defence).is_equal(captain.defence)
	assert_float(unit.critical).is_equal(captain.critical)
	assert_bool(unit.parries).is_true()
	(
		assert_bool(FormationUnits.make(load("res://content/units/grem.tres"), "p", 1, 2).parries)
		. is_false()
	)
