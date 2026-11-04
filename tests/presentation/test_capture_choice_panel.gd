extends GdUnitTestSuite
## Per specs/08: each button method calls exactly one issue_choice() and nothing else -
## verified against the crate's economy (`EconomyState`, spec 39 step 9b-ii), same
## reasoning as test_time_controls.gd.

const CaptureChoicePanel = preload("res://presentation/capture_choice_panel.gd")
const NodeDef = preload("res://content/definitions/node_def.gd")


func _resource_node() -> NodeDef:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.RESOURCE
	node.yield_food_per_tick = 6
	node.decay_interval_ticks = 5
	node.decay_floor_food = 2
	node.ravage_yield_food = 40
	return node


func _fort_node() -> NodeDef:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.FORT
	node.dismantle_wood_yield = 8
	node.dismantle_stone_yield = 5
	node.fortify_wood_cost = 10
	node.fortify_stone_cost = 6
	return node


func test_ravage_pressed_grants_the_lump_sum() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	var resolution := economy
	var node := _resource_node()
	resolution.on_node_captured(0, node)
	var panel: CaptureChoicePanel = auto_free(CaptureChoicePanel.new())
	panel.capture_resolution = resolution

	panel.on_ravage_pressed(0)

	assert_int(economy.balance("food")).is_equal(40)


func test_dismantle_pressed_grants_salvage() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	var resolution := economy
	var node := _fort_node()
	resolution.on_node_captured(1, node)
	var panel: CaptureChoicePanel = auto_free(CaptureChoicePanel.new())
	panel.capture_resolution = resolution

	panel.on_dismantle_pressed(1)

	assert_int(economy.balance("wood")).is_equal(8)
	assert_int(economy.balance("stone")).is_equal(5)
	assert_bool(resolution.is_awaiting_choice(1)).is_false()


func test_fortify_pressed_spends_the_cost_when_affordable() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("wood", 10)
	economy.add("stone", 6)
	var resolution := economy
	var node := _fort_node()
	resolution.on_node_captured(1, node)
	var panel: CaptureChoicePanel = auto_free(CaptureChoicePanel.new())
	panel.capture_resolution = resolution

	panel.on_fortify_pressed(1)

	assert_int(economy.balance("wood")).is_equal(0)
	assert_int(economy.balance("stone")).is_equal(0)
	assert_bool(resolution.is_awaiting_choice(1)).is_false()
