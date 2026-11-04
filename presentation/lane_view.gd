class_name LaneView
extends Node2D
## Renders the lane and interpolates unit positions between tick boundaries
## (Decision 1), per specs/05-presentation-and-hud.md. Read-only: only ever reads
## simulation state via `lane`'s query methods; never mutates simulation state.
##
## Self-subscribes to SimEvents.tick_advanced in _ready() - unlike sim/'s RefCounted
## components, a Node's scene-tree lifecycle handles connection/disconnection
## automatically (Godot disconnects a Node's signal connections when it exits the
## tree), so the reference-lifetime concern that ruled out self-subscription for
## sim/ classes doesn't apply here.
##
## Scoped to this slice's actual content shape - at most one player wave and one
## moving blocker (Hero Party) at a time, per specs/00's scripted scenario - tracked
## as two named positions rather than a generic multi-entity id scheme. A future map
## with simultaneous waves can tell them apart by the `id` each of the lane's waves and
## moving blockers carries (`LaneState`, native/rust/src/lane/binding.rs).
##
## Honest test-coverage note (see specs/05's own Notes): TickInterpolation's pure
## math is unit-tested headlessly (tests/presentation/test_tick_interpolation.gd).
## This class's actual rendering has no automated coverage - it needs a real render
## context, which only exists once a running scene is assembled (Phase 3 item 11).
## Manual playtest verifies it there.
##
## speed_multiplier (found necessary via manual playtest) scales
## _elapsed_since_last_tick's own accumulation the same way SimulationClock scales
## its internal elapsed counter - without this, the unit's on-screen animation filled
## at the old 1x pace while the real tick fired early at 2x/4x, cutting the animation
## short (visibly stopping halfway at 2x, a quarter of the way at 4x) instead of
## finishing right as the tick advanced.

const TickInterpolation = preload("res://presentation/tick_interpolation.gd")

var lane: Object
var tick_duration_seconds: float = 1.0
var speed_multiplier: float = 1.0
var node_spacing: float = 96.0

var _elapsed_since_last_tick: float = 0.0
var _previous_positions: Dictionary = {}
var _current_positions: Dictionary = {}


func _ready() -> void:
	SimEvents.tick_advanced.connect(_on_tick_advanced)
	_current_positions = _snapshot_positions()
	_previous_positions = _current_positions.duplicate()


func _process(delta: float) -> void:
	_elapsed_since_last_tick += delta * speed_multiplier
	queue_redraw()


func _draw() -> void:
	if lane == null:
		return
	var fraction := TickInterpolation.elapsed_fraction(
		_elapsed_since_last_tick, tick_duration_seconds
	)
	for entity_name in _current_positions.keys():
		var previous: float = _previous_positions.get(entity_name, _current_positions[entity_name])
		var current: float = _current_positions[entity_name]
		var interpolated := TickInterpolation.interpolate_position(previous, current, fraction)
		var color := Color.ORANGE if entity_name == "player_wave" else Color.PURPLE
		draw_circle(Vector2(interpolated * node_spacing, 0.0), 8.0, color)


func snap_to_current_tick() -> void:
	# Called by time controls (item 10) right after SimulationClock.skip_to_next_marker(),
	# whose own tick_advanced emission already ran _on_tick_advanced() below and reset
	# elapsed to 0 - which would render the *previous* tick's position at fraction=0,
	# then visibly animate forward over the next tick_duration_seconds. Per this spec's
	# own scenario ("immediately reflects the new tick's full state with no visible
	# half-interpolated frame persisting"), a skip should show the new state at once
	# instead: forcing elapsed to the full tick duration makes the very next _draw()
	# compute fraction=1.0 (current_positions), not restart a fresh lerp from 0.
	_elapsed_since_last_tick = tick_duration_seconds
	queue_redraw()


func _on_tick_advanced(_tick_number: int) -> void:
	_elapsed_since_last_tick = 0.0
	_previous_positions = _current_positions.duplicate()
	_current_positions = _snapshot_positions()
	queue_redraw()


func _snapshot_positions() -> Dictionary:
	var positions := {}
	if lane.waves().size() > 0:
		positions["player_wave"] = float(lane.waves()[0]["position"])
	if lane.moving_blockers().size() > 0:
		positions["hero_party"] = float(lane.moving_blockers()[0]["position"])
	return positions
