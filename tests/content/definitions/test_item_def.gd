extends GdUnitTestSuite
## Items, per Decision 120 (spec 28): each takes the slots it names on its carrier's body, and
## an innate weapon can't strike while a carried item holds its body part. What presentation
## reads of them (spec 39 step 9e-ii).

const ItemDef = preload("res://content/definitions/item_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _unit(slots: Array[String], items: Array[ItemDef]) -> UnitDef:
	var unit := UnitDef.new()
	unit.hp = 10
	unit.slots = slots
	unit.items = items
	return unit


func _item(slots: Array[String]) -> ItemDef:
	var item := ItemDef.new()
	item.slots = slots
	return item


func test_a_carried_item_in_its_body_part_puts_an_innate_weapon_out_of_use() -> void:
	var fists := WeaponDef.innate_weapon(2)
	fists.slots = ["hand"]
	var bare := _unit(["hand", "hand"], [fists])
	var armed := _unit(["hand", "hand"], [_spear(), fists])
	var one_handed := _unit(["hand", "hand"], [_item(["hand"]), fists])

	assert_int(bare.melee_damage()).is_equal(2)
	assert_int(armed.melee_damage()).is_equal(5)  # the spear; its fists are full
	assert_int(one_handed.melee_damage()).is_equal(2)  # a hand still free


func _spear() -> WeaponDef:
	var spear := WeaponDef.new()
	spear.damage = 5
	spear.slots = ["hand", "hand"]
	return spear


func test_the_units_in_play_carry_items_that_fit() -> void:
	for name in ["grem", "grem_brute", "grem_chieftain", "grem_spitter", "kingdom_militia"]:
		var unit: UnitDef = CrateContent.unit(name)
		assert_bool(unit.items.is_empty()).is_false()
		assert_int(unit.melee_damage()).is_greater(0)  # it fights with its innate weapons
