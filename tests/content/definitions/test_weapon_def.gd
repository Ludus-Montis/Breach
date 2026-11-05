extends GdUnitTestSuite
## WeaponDef, per Decision 47: a unit's damage comes from what it fights with.

const WeaponDef = preload("res://content/definitions/weapon_def.gd")


func _weapon(damage: int, attack_range: int) -> WeaponDef:
	var weapon := WeaponDef.new()
	weapon.item_name = "claw"
	weapon.damage = damage
	weapon.damage_type = "slashing"
	weapon.attack_range = attack_range
	return weapon


func test_a_weapon_without_range_is_melee() -> void:
	assert_bool(_weapon(3, 0).is_melee()).is_true()
	assert_bool(_weapon(4, 5).is_melee()).is_false()


func test_traits_carry_siege() -> void:
	var fists := _weapon(7, 0)
	fists.traits = {"siege": 1}

	assert_int(fists.traits.get("siege", 0)).is_equal(1)
