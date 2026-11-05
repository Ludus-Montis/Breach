extends GdUnitTestSuite
## Per specs/05's "Time controls affect timing only, never simulation content"
## scenario: each button method calls exactly one SimulationClock method and nothing
## else - verified here with the crate's SimulationClock (spec 39 step 9c) rather than a mock,
## since it's cheap to make.

const TimeControls = preload("res://presentation/time_controls.gd")


func test_pause_button_pauses_when_not_paused() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_pause_pressed()

	assert_bool(clock.is_paused()).is_true()


func test_pause_button_resumes_when_already_paused() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	clock.pause()
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_pause_pressed()

	assert_bool(clock.is_paused()).is_false()


func test_skip_button_advances_exactly_one_tick() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_skip_pressed()

	assert_int(clock.tick_number()).is_equal(1)


func test_skip_button_works_even_while_paused() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	clock.pause()
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_skip_pressed()

	assert_int(clock.tick_number()).is_equal(1)


func test_speed_selected_sets_the_multiplier_and_nothing_else() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_speed_selected(4.0)

	assert_float(clock.speed_multiplier).is_equal_approx(4.0, 0.001)
	assert_int(clock.tick_number()).is_equal(0)
	assert_bool(clock.is_paused()).is_false()


func test_auto_pause_toggled_on_sets_the_flag() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_auto_pause_toggled(true)

	assert_bool(clock.auto_pause_each_tick).is_true()


func test_auto_pause_toggled_off_sets_the_flag() -> void:
	var clock: Object = NativeLibrary.made("SimulationClock")
	clock.auto_pause_each_tick = true
	var controls: TimeControls = auto_free(TimeControls.new())
	controls.simulation_clock = clock

	controls.on_auto_pause_toggled(false)

	assert_bool(clock.auto_pause_each_tick).is_false()
