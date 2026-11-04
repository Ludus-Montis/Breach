extends GdUnitTestSuite
## LaneView's actual rendering has no automated coverage - it needs a real render
## context, only available once a running scene exists (Phase 3 item 11); manual
## playtest verifies it there, per specs/05's own Notes. This smoke test only
## confirms the script compiles and constructs cleanly (catches a type error like the
## one TickInterpolation actually hit while writing this item), not its behavior.

const LaneView = preload("res://presentation/lane_view.gd")
const TickInterpolation = preload("res://presentation/tick_interpolation.gd")
const NodeDef = preload("res://content/definitions/node_def.gd")


func test_constructs_without_error() -> void:
	var view: LaneView = auto_free(LaneView.new())

	assert_object(view).is_not_null()


func test_snapshot_positions_reflects_the_lane_state() -> void:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.ORIGIN
	var nodes: Array[NodeDef] = [node, node]
	var lane: Object = NativeLibrary.made("LaneState")
	lane.start(nodes)
	lane.spawn_wave("player", [{"hp": 10, "dmg": 2}], 0, 1)

	var view: LaneView = auto_free(LaneView.new())
	view.lane = lane
	var positions: Dictionary = view._snapshot_positions()

	assert_float(positions["player_wave"]).is_equal_approx(0.0, 0.001)
	assert_bool(positions.has("hero_party")).is_false()


func test_snap_to_current_tick_forces_full_elapsed_so_the_next_frame_shows_the_new_state() -> void:
	var view: LaneView = auto_free(LaneView.new())
	view.tick_duration_seconds = 2.0
	view._elapsed_since_last_tick = 0.1  # mid-interpolation, as if just after a tick

	view.snap_to_current_tick()

	var fraction := TickInterpolation.elapsed_fraction(
		view._elapsed_since_last_tick, view.tick_duration_seconds
	)
	assert_float(fraction).is_equal_approx(1.0, 0.001)
