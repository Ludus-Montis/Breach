extends GdUnitTestSuite
## The crate's clocks (`SimulationClock`, `SkirmishClock`, native/rust/src/clock/binding.rs,
## spec 39 step 9c) as Godot reads them. Their rules are the crate's cargo tests
## (src/clock/tests.rs); these hold what only Godot sees: the properties GDScript sets, a tick
## told on the object given, and a handler's calls back while it is.

var _clock: Object


func before_test() -> void:
	_clock = NativeLibrary.made("SimulationClock")


func after_test() -> void:
	for connection in SimEvents.tick_advanced.get_connections():
		if connection["callable"].get_object() == self:
			SimEvents.tick_advanced.disconnect(connection["callable"])


func _pause_on_tick(_tick: int) -> void:
	_clock.pause()


func _auto_pause_on_tick(_tick: int) -> void:
	_clock.auto_pause_each_tick = true


func test_its_settings_start_as_the_gdscript_fields_did() -> void:
	assert_float(_clock.tick_duration_seconds).is_equal(1.0)
	assert_float(_clock.speed_multiplier).is_equal(1.0)
	assert_bool(_clock.auto_pause_each_tick).is_false()


func test_the_settings_set_pace_a_frame() -> void:
	_clock.tick_duration_seconds = 2.0
	_clock.speed_multiplier = 2.0

	_clock.advance_time(1.0)

	assert_int(_clock.tick_number()).is_equal(1)


func test_a_tick_is_told_on_the_object_given() -> void:
	_clock.emit_on(SimEvents)
	monitor_signals(SimEvents, false)

	_clock.advance_tick()

	await assert_signal(SimEvents).is_emitted("tick_advanced", 1)


func test_a_tick_is_told_by_a_frame_and_by_a_skip() -> void:
	_clock.emit_on(SimEvents)
	monitor_signals(SimEvents, false)

	_clock.advance_time(1.0)
	_clock.skip_to_next_marker()

	await assert_signal(SimEvents).is_emitted("tick_advanced", 2)


func test_a_handler_may_pause_it_while_the_tick_is_told() -> void:
	_clock.emit_on(SimEvents)
	SimEvents.tick_advanced.connect(_pause_on_tick)

	_clock.advance_tick()

	assert_bool(_clock.is_paused()).is_true()


func test_auto_pause_is_read_after_the_handlers() -> void:
	_clock.emit_on(SimEvents)
	SimEvents.tick_advanced.connect(_auto_pause_on_tick)

	_clock.skip_to_next_marker()

	assert_bool(_clock.is_paused()).is_true()


func test_the_skirmish_clock_carries_its_leftover_by_the_tick_set() -> void:
	var clock: Object = NativeLibrary.made("SkirmishClock")
	assert_float(clock.tick_seconds).is_equal(0.1)
	clock.tick_seconds = 0.25

	var due: int = clock.advance(0.3) + clock.advance(0.3)

	assert_int(due).is_equal(2)
	assert_float(clock.fraction()).is_equal_approx(0.4, 0.001)
