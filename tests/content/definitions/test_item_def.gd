extends GdUnitTestSuite
## Items, per Decision 120 (spec 28): one group - weapons, armour, tools - each taking the
## slots it names on its carrier's body, with a weight, a strength needed and tags; a tool
## grants its traits to whoever carries it.

const ItemDef = preload("res://content/definitions/item_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _unit(slots: Array[String], items: Array[ItemDef]) -> UnitDef:
	var unit := UnitDef.new()
	unit.hp = 10
	unit.speed = 1.0
	unit.slots = slots
	unit.items = items
	return unit


func _item(item_name: String, slots: Array[String], grants := {}) -> ItemDef:
	var item := ItemDef.new()
	item.item_name = item_name
	item.slots = slots
	item.grants = grants
	return item


func test_a_carried_item_in_its_body_part_puts_an_innate_weapon_out_of_use() -> void:
	var fists := WeaponDef.innate_weapon(2)
	fists.slots = ["hand"]
	var spit := WeaponDef.innate_weapon(4, "spit")
	spit.slots = ["mouth"]
	spit.attack_range = 5
	var bare := _unit(["hand", "hand", "mouth"], [fists, spit])
	var armed := _unit(["hand", "hand", "mouth"], [_spear(), fists, spit])
	var one_handed := _unit(["hand", "hand", "mouth"], [_item("shield", ["hand"]), fists])

	assert_int(bare.melee_damage()).is_equal(2)
	assert_int(armed.melee_damage()).is_equal(5)  # the spear; its fists are full
	assert_int(one_handed.melee_damage()).is_equal(2)  # a hand still free
	assert_str(armed.ranged_weapon().item_name).is_equal("spit")  # its mouth is free
	armed.items.append(_item("gag", ["mouth"]))
	assert_object(armed.ranged_weapon()).is_null()


func _spear() -> WeaponDef:
	var spear := WeaponDef.new()
	spear.item_name = "spear"
	spear.damage = 5
	spear.slots = ["hand", "hand"]
	return spear


func test_an_item_grants_its_grants_but_its_own_traits_are_its_own() -> void:
	var shovel := _item("shovel", ["hand"], {"burrower": 1})
	var ram := WeaponDef.new()
	ram.item_name = "ram"
	ram.traits = {"siege": 2}
	var unit := _unit(["hand"], [shovel, ram])
	unit.traits = {"burrower": 0, "climber": 2}

	assert_int(unit.trait_level("burrower")).is_equal(1)
	assert_int(unit.trait_level("climber")).is_equal(2)
	assert_int(unit.trait_level("siege")).is_equal(0)


func test_the_units_in_play_carry_items_that_fit() -> void:
	for name in ["grem", "grem_brute", "grem_chieftain", "grem_spitter", "kingdom_militia"]:
		var unit: UnitDef = CrateContent.unit(name)
		assert_bool(unit.items.is_empty()).is_false()
		assert_int(unit.melee_damage()).is_greater(0)  # it fights with its innate weapons
