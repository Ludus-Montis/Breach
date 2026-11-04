class_name TimeControls
extends Control
## Play/pause, skip-to-marker, speed, and auto-pause, per
## specs/05-presentation-and-hud.md. Each method touches exactly one SimulationClock
## field/method and nothing else - the spec's "affect timing only, never simulation
## content" scenario. Button wiring itself (button.pressed.connect(...)) is untested
## engine glue, same honesty as LaneView's _ready/_process/_draw - these on_x()
## methods are what's actually verified.

## The crate's SimulationClock (spec 39 step 9c).
var simulation_clock: Object
var lane_view: LaneView


func on_pause_pressed() -> void:
	if simulation_clock.is_paused():
		simulation_clock.resume()
	else:
		simulation_clock.pause()


func on_skip_pressed() -> void:
	simulation_clock.skip_to_next_marker()
	if lane_view != null:
		lane_view.snap_to_current_tick()


func on_speed_selected(multiplier: float) -> void:
	simulation_clock.speed_multiplier = multiplier


func on_auto_pause_toggled(enabled: bool) -> void:
	simulation_clock.auto_pause_each_tick = enabled
