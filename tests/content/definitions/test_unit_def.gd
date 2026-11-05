extends GdUnitTestSuite
## UnitDef, what presentation reads of a unit (spec 39 step 9e-ii): its melee strike's damage.

const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")


func test_melee_damage_is_the_sum_of_its_melee_weapons() -> void:
	var unit := UnitDef.new()
	unit.items = [_weapon(3, 0), _weapon(3, 0), _weapon(4, 5)]

	assert_int(unit.melee_damage()).is_equal(6)


func test_a_unit_strikes_with_its_innate_weapon_and_without_one_not_at_all() -> void:
	var fists := UnitDef.new()
	fists.items = [WeaponDef.innate_weapon(9)]

	assert_int(fists.melee_damage()).is_equal(9)
	assert_int(UnitDef.new().melee_damage()).is_equal(0)


func _weapon(damage: int, attack_range: int) -> WeaponDef:
	var weapon := WeaponDef.new()
	weapon.damage = damage
	weapon.attack_range = attack_range
	return weapon
