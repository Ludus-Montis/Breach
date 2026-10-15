extends GdUnitTestSuite
## A unit's sheet (Decision 117, spec 28): archetype tags, five attributes defaulting to
## the average, rated traits; a battle unit carries its own copy, so one unit can change
## without its type.

const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.speed = 1.0
	return unit_def


func test_attributes_default_to_the_average_and_must_not_be_negative() -> void:
	var unit_def := _def()

	for attribute in UnitDef.ATTRIBUTES:
		assert_int(unit_def.get(attribute)).is_equal(UnitDef.AVERAGE)
	assert_array(unit_def.validate()).is_empty()
	unit_def.agility = -1
	assert_bool(Array(unit_def.validate()).any(func(e): return e.contains("agility"))).is_true()


func test_traits_are_rated_and_a_missing_one_is_level_zero() -> void:
	var unit_def := _def()
	unit_def.traits = {"climber": 2}

	assert_int(unit_def.trait_level("climber")).is_equal(2)
	assert_int(unit_def.trait_level("swimmer")).is_equal(0)
	unit_def.traits = {"climber": -1}
	assert_array(unit_def.validate()).is_not_empty()


func test_a_battle_unit_carries_its_own_copy_of_the_sheet() -> void:
	var unit_def := _def()
	unit_def.tags = ["human", "martial"]
	unit_def.strength = 14
	unit_def.traits = {"darksight": 1}

	var unit := FormationUnits.make(unit_def, "player", 1, 1)
	unit.traits["darksight"] = 3
	unit.tags.append("veteran")

	assert_array(unit.tags).contains(["human", "martial"])
	assert_int(unit.attributes["strength"]).is_equal(14)
	assert_int(unit.attributes["wits"]).is_equal(UnitDef.AVERAGE)
	assert_int(unit_def.trait_level("darksight")).is_equal(1)  # its type is untouched
	assert_array(unit_def.tags).not_contains(["veteran"])


func test_the_units_in_play_carry_archetype_tags() -> void:
	for name in ["grem", "grem_brute", "grem_chieftain", "grem_spitter"]:
		var unit_def: UnitDef = load("res://content/units/%s.tres" % name)
		assert_array(unit_def.tags).contains(["grem"])
		assert_array(unit_def.validate()).is_empty()
	for name in ["kingdom_militia", "kingdom_captain"]:
		var unit_def: UnitDef = load("res://content/units/%s.tres" % name)
		assert_array(unit_def.tags).contains(["human", "martial"])


func test_a_tool_carried_grants_its_traits_to_the_battle_unit() -> void:
	var shovel := ItemDef.new()
	shovel.item_name = "shovel"
	shovel.slots = ["hand"]
	shovel.grants = {"burrower": 1}
	var unit_def := _def()
	unit_def.slots = ["hand"]
	unit_def.items = [shovel]

	var unit := FormationUnits.make(unit_def, "player", 1, 1)

	assert_int(unit.traits.get("burrower", 0)).is_equal(1)
	assert_dict(unit_def.traits).is_empty()  # its type's own traits are untouched
