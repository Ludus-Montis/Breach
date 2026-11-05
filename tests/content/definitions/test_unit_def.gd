extends GdUnitTestSuite

const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")


func test_a_unit_is_one_slot_by_default() -> void:
	var unit := UnitDef.new()

	assert_int(unit.footprint_depth).is_equal(1)
	assert_int(unit.footprint_width).is_equal(1)


func test_a_unit_prefers_the_front_by_default() -> void:
	var unit := UnitDef.new()

	assert_int(unit.preferred_position).is_equal(UnitDef.Position.FRONT)


func test_melee_damage_is_the_sum_of_its_melee_weapons() -> void:
	var unit := UnitDef.new()
	unit.items = [_weapon("bite", 3, 0), _weapon("claw", 3, 0), _weapon("spit", 4, 5)]

	assert_int(unit.melee_damage()).is_equal(6)
	assert_str(unit.ranged_weapon().item_name).is_equal("spit")


func test_a_unit_strikes_with_its_innate_weapon_and_without_one_not_at_all() -> void:
	var fists := UnitDef.new()
	fists.items = [WeaponDef.innate_weapon(9)]

	assert_int(fists.melee_damage()).is_equal(9)
	assert_object(fists.ranged_weapon()).is_null()
	assert_int(UnitDef.new().melee_damage()).is_equal(0)


func _weapon(weapon_name: String, damage: int, attack_range: int) -> WeaponDef:
	var weapon := WeaponDef.new()
	weapon.item_name = weapon_name
	weapon.damage = damage
	weapon.attack_range = attack_range
	return weapon
