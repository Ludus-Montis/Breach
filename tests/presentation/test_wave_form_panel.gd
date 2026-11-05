extends GdUnitTestSuite
## Per specs/08's wave-forming input scenarios. The crate's economy (`EconomyState`) and
## a real CommandQueue throughout (cheap to construct), same reasoning as test_time_controls.gd.

const WaveFormPanel = preload("res://presentation/wave_form_panel.gd")
const CommandQueue = preload("res://sim/command_queue.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


func _grem(cost_food: int = 8) -> UnitDef:
	var unit := UnitDef.new()
	unit.cost_food = cost_food
	unit.hp = 20
	unit.items = [WeaponDef.innate_weapon(6)]
	return unit


func test_queueing_an_affordable_grem_spends_food_and_returns_true() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 10)
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = CommandQueue.new()

	var result := panel.on_queue_grem_pressed(_grem(8))

	assert_bool(result).is_true()
	assert_int(economy.balance("food")).is_equal(2)


func test_queueing_an_unaffordable_grem_spends_nothing_and_returns_false() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 5)
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = CommandQueue.new()

	var result := panel.on_queue_grem_pressed(_grem(8))

	assert_bool(result).is_false()
	assert_int(economy.balance("food")).is_equal(5)


func test_marching_with_no_queued_units_does_nothing() -> void:
	var queue := CommandQueue.new()
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = NativeLibrary.made("EconomyState")
	panel.command_queue = queue

	panel.on_march_pressed(0, 1)

	assert_array(queue.pending_commands()).is_empty()


func test_marching_with_queued_units_enqueues_one_filled_command() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 8)
	var queue := CommandQueue.new()
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = queue
	panel.on_queue_grem_pressed(_grem(8))

	panel.on_march_pressed(0, 1)

	assert_array(queue.pending_commands()).has_size(1)
	var id: int = queue.pending_commands()[0]
	assert_bool(queue.is_filled(id)).is_true()


func test_marching_clears_the_buffer_for_the_next_wave() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 16)
	var queue := CommandQueue.new()
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = queue
	panel.on_queue_grem_pressed(_grem(8))
	panel.on_march_pressed(0, 1)

	panel.on_march_pressed(0, 1)

	assert_array(queue.pending_commands()).has_size(1)


func test_pending_count_reflects_queued_units() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 16)
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = CommandQueue.new()

	panel.on_queue_grem_pressed(_grem(8))
	panel.on_queue_grem_pressed(_grem(8))

	assert_int(panel.pending_count()).is_equal(2)


func test_marching_resets_pending_count_to_zero() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 8)
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = CommandQueue.new()
	panel.on_queue_grem_pressed(_grem(8))

	panel.on_march_pressed(0, 1)

	assert_int(panel.pending_count()).is_equal(0)


func test_queueing_beyond_the_cap_does_nothing_and_returns_false() -> void:
	var economy: Object = NativeLibrary.made("EconomyState")
	economy.add("food", 100)
	var panel: WaveFormPanel = auto_free(WaveFormPanel.new())
	panel.economy = economy
	panel.command_queue = CommandQueue.new()
	for i in range(panel.max_pending_units):
		panel.on_queue_grem_pressed(_grem(8))

	var result := panel.on_queue_grem_pressed(_grem(8))

	assert_bool(result).is_false()
	assert_int(panel.pending_count()).is_equal(panel.max_pending_units)
	assert_int(economy.balance("food")).is_equal(100 - panel.max_pending_units * 8)
