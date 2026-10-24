extends GdUnitTestSuite
## Load, per spec 28 part 7 (Decisions 117 and 120): what a unit carries against its
## strength sets its encumbrance - slower, tiring faster, dodging worse - and its most
## stamina is from its constitution. Stamina spent and regained, and the pace and skill a
## tired unit has, are the crate's (native/rust/src/stamina/tests.rs, spec 39 step 6a).

const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")


## A unit of strength 10 (carry limit 20 with the usual tuning) carrying `weight`.
func _laden(weight: float, hauler: int = 0) -> UnitDef:
	var pack := ItemDef.new()
	pack.item_name = "pack"
	pack.weight = weight
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.speed = 1.0
	unit_def.items = [pack]
	if hauler > 0:
		unit_def.traits = {"hauler": hauler}
	return unit_def


func test_its_load_against_its_strength_sets_its_encumbrance() -> void:
	assert_int(UnitArms.load_stage(_laden(10.0))).is_equal(0)
	assert_int(UnitArms.load_stage(_laden(15.0))).is_equal(1)
	assert_int(UnitArms.load_stage(_laden(30.0))).is_equal(2)
	assert_int(UnitArms.load_stage(_laden(45.0))).is_equal(3)
	assert_int(UnitArms.load_stage(_laden(45.0, 1))).is_equal(2)  # a hauler carries more


func test_a_laden_unit_is_slower_tires_faster_and_dodges_worse() -> void:
	var tuning := BattleTuning.current()
	var light := FormationUnits.make(_laden(5.0), "player", 1, 1)
	var heavy := FormationUnits.make(_laden(30.0), "player", 1, 2)

	assert_float(light.fresh_speed).is_equal(1.0)
	assert_float(heavy.fresh_speed).is_equal_approx(tuning.load_pace[2], 0.0001)
	assert_float(heavy.tiring).is_greater(light.tiring)
	assert_float(heavy.dodging).is_less(light.dodging)
	assert_float(light.max_stamina).is_equal(10 * tuning.stamina_per_constitution)
