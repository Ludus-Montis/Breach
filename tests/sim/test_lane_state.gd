extends GdUnitTestSuite
## The lane held by the crate (`LaneState`, native/rust/src/lane/binding.rs, spec 39 step
## 9b-i) as Godot reads it. Its rules are the crate's cargo tests (src/lane/tests.rs); these
## hold what only Godot sees: each wave's events are emitted before the next arrives, so a
## handler that calls back into the lane acts between arrivals, as it did in GDScript, and a
## moving blocker's dictionary stays its truth.

const NodeDef = preload("res://content/definitions/node_def.gd")

var _lane: Object
var _on_captured: Callable


func before_test() -> void:
	_lane = NativeLibrary.made("LaneState")
	var nodes: Array[NodeDef] = []
	for index in 4:
		var node := NodeDef.new()
		node.node_type = NodeDef.NodeType.ORIGIN
		nodes.append(node)
	_lane.start(nodes)


func after_test() -> void:
	if _on_captured.is_valid() and SimEvents.node_captured.is_connected(_on_captured):
		SimEvents.node_captured.disconnect(_on_captured)


func _spawn_a_mover_on_taking_node_1(node: int) -> void:
	if node == 1:
		_lane.spawn_moving_blocker("defender", {"hp": 50, "dmg": 1}, 2, 0)


func _harvest_on_taking(node: int) -> void:
	_lane.harvest(node, "player")


func test_a_wave_s_events_are_emitted_on_the_object_given() -> void:
	monitor_signals(SimEvents, false)
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 2}], 0, 1)

	_lane.advance_positions(SimEvents)

	assert_str(_lane.node_state(1)["owner"]).is_equal("player")
	await assert_signal(SimEvents).is_emitted("node_captured", 1)


func test_a_mover_a_handler_spawns_meets_a_later_wave_in_the_same_tick() -> void:
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 2}], 0, 1)
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 2}], 1, 1)
	_on_captured = _spawn_a_mover_on_taking_node_1
	SimEvents.node_captured.connect(_on_captured)

	_lane.advance_positions(SimEvents)

	assert_str(_lane.node_state(2)["owner"]).is_equal("")
	assert_int(_lane.moving_blockers()[0]["blocker"]["hp"]).is_equal(48)


func test_a_handler_s_harvest_counts_the_wave_and_takes_it_off_the_lane() -> void:
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 2}, {"hp": 10, "dmg": 2}], 0, 1)
	_on_captured = _harvest_on_taking
	SimEvents.node_captured.connect(_on_captured)

	_lane.advance_positions(SimEvents)

	assert_array(_lane.waves()).is_empty()
	assert_int(_lane.units_of("player")).is_equal(2)


func test_a_moving_blocker_s_dictionary_holds_each_clash_s_hp() -> void:
	var blocker := {"hp": 50, "dmg": 3}
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 6}], 0, 1)
	_lane.spawn_moving_blocker("defender", blocker, 1, 0)

	_lane.advance_positions(SimEvents)

	assert_int(blocker["hp"]).is_equal(44)


func test_a_moving_blocker_fights_with_what_its_dictionary_was_set_to() -> void:
	var blocker := {"hp": 50, "dmg": 3}
	_lane.spawn_wave("player", [{"hp": 10, "dmg": 6}], 0, 1)
	_lane.spawn_moving_blocker("defender", blocker, 1, 0)
	blocker["hp"] = 20

	_lane.advance_positions(SimEvents)

	assert_int(_lane.moving_blockers()[0]["blocker"]["hp"]).is_equal(14)


func test_a_despawned_wave_named_by_its_dictionary_is_off_the_lane() -> void:
	var wave: Dictionary = _lane.spawn_wave("player", [{"hp": 10, "dmg": 2}], 0, 1)

	_lane.despawn_wave(wave)

	assert_array(_lane.waves()).is_empty()
