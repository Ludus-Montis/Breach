extends GdUnitTestSuite
## UnitArms and what a unit takes into the field (Decisions 118 to 120, spec 28 parts 4 and
## 5): strength scales a weapon over its requirement, a unit wields worse untrained or too
## weak, armour worn adds to its own, it parries with a held weapon or a shield, and a unit
## in the field carries its skills. How its blows land is the crate's (`landing.rs`).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const ArmourDef = preload("res://content/definitions/armour_def.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")


func test_a_unit_parries_with_a_held_weapon_or_a_shield_not_its_claws() -> void:
	var claws := UnitDef.new()
	claws.items = [WeaponDef.innate_weapon(3, "claw")]
	var shield := ItemDef.new()
	shield.item_name = "shield"
	shield.tags = ["shield"]
	var shielded := UnitDef.new()
	shielded.items = [WeaponDef.innate_weapon(3, "claw"), shield]
	var militia: UnitDef = CrateContent.unit("kingdom_militia")

	assert_bool(claws.can_parry()).is_false()
	assert_bool(shielded.can_parry()).is_true()
	assert_bool(militia.can_parry()).is_true()


func test_a_unit_in_the_field_carries_its_skills() -> void:
	var captain: UnitDef = CrateContent.unit("kingdom_captain")
	var unit := FormationUnits.make(captain, "the_kingdom", -1, 1)

	assert_int(unit.melee_skill).is_equal(
		captain.melee_skill + roundi(BattleTuning.current().item_mastered_skill)
	)  # a mastered polearm
	assert_int(unit.defence).is_equal(captain.defence)
	assert_float(unit.critical).is_equal(captain.critical)
	assert_bool(unit.parries).is_true()
	assert_bool(FormationUnits.make(CrateContent.unit("grem"), "p", 1, 2).parries).is_false()


func test_strength_scales_a_weapon_over_its_requirement() -> void:
	var maul := WeaponDef.new()
	maul.item_name = "maul"
	maul.damage = 8
	maul.strength_requirement = 12
	maul.strength_scaling = 0.5
	var ogre := UnitDef.new()
	ogre.strength = 16
	ogre.items = [maul]
	var weakling := UnitDef.new()
	weakling.strength = 9
	weakling.items = [maul]

	assert_float(UnitArms.parts(ogre, false)[0][0]).is_equal(10.0)
	assert_float(UnitArms.parts(weakling, false)[0][0]).is_equal(8.0)
	var lost := roundi(3 * BattleTuning.current().item_weak_skill)
	assert_int(UnitArms.skill(weakling, false)).is_equal(weakling.melee_skill - lost)


func test_proficiency_shifts_the_skill_a_weapon_is_wielded_at() -> void:
	var axe := WeaponDef.new()
	axe.item_name = "axe"
	axe.damage = 6
	axe.tags = ["axe"]
	var unit := UnitDef.new()
	unit.items = [axe]
	var tuning := BattleTuning.current()

	assert_int(UnitArms.skill(unit, false)).is_equal(
		roundi(unit.melee_skill + tuning.item_untrained_skill)
	)
	unit.proficiencies = {"axe": 1}
	assert_int(UnitArms.skill(unit, false)).is_equal(unit.melee_skill)
	unit.proficiencies = {"axe": 2}
	assert_int(UnitArms.skill(unit, false)).is_equal(
		roundi(unit.melee_skill + tuning.item_mastered_skill)
	)


func test_armour_worn_adds_to_its_own() -> void:
	var gambeson := ArmourDef.new()
	gambeson.item_name = "gambeson"
	gambeson.armour = 1
	var unit := UnitDef.new()
	unit.armour = 1
	unit.ward = 2
	unit.items = [gambeson]

	assert_object(UnitArms.protection(unit)).is_equal(Vector2i(2, 2))
