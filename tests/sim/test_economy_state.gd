extends GdUnitTestSuite
## The economy held by the crate (`EconomyState`, native/rust/src/economy/binding.rs, spec 39
## step 9b-ii) as Godot reads it. Its rules are the crate's cargo tests
## (src/economy/tests.rs); these hold what only Godot sees: a NodeDef's kind and figures are
## read across, and a choice is named by its word.

const NodeDef = preload("res://content/definitions/node_def.gd")

var _economy: Object


func before_test() -> void:
	_economy = NativeLibrary.made("EconomyState")


func _farm() -> NodeDef:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.RESOURCE
	node.yield_food_per_tick = 10
	node.decay_interval_ticks = 2
	node.decay_floor_food = 2
	node.ravage_yield_food = 40
	return node


func _fort() -> NodeDef:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.FORT
	node.dismantle_wood_yield = 8
	node.dismantle_stone_yield = 5
	node.fortify_wood_cost = 10
	node.fortify_stone_cost = 6
	return node


func test_a_farm_s_yield_and_its_decay_are_read_from_its_node_def() -> void:
	_economy.on_node_captured(1, _farm())

	_economy.on_tick_advanced(1)
	_economy.on_tick_advanced(2)

	# 10 on the first tick; one interval of 2 in, 9 on the second.
	assert_int(_economy.balance("food")).is_equal(19)


func test_a_fort_s_choice_is_named_by_its_word() -> void:
	_economy.add("wood", 10)
	_economy.add("stone", 6)
	_economy.on_node_captured(2, _fort())

	_economy.issue_choice(2, "raze")
	var awaiting_after_an_unknown_word: bool = _economy.is_awaiting_choice(2)
	_economy.issue_choice(2, "fortify")

	assert_bool(awaiting_after_an_unknown_word).is_true()
	assert_bool(_economy.is_awaiting_choice(2)).is_false()
	assert_int(_economy.balance("wood")).is_equal(0)


func test_a_farm_ravaged_gives_its_lump_sum_once() -> void:
	_economy.on_node_captured(1, _farm())

	_economy.issue_choice(1, "ravage")
	_economy.issue_choice(1, "ravage")
	_economy.on_tick_advanced(1)

	assert_int(_economy.balance("food")).is_equal(40)
