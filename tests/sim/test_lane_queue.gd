extends GdUnitTestSuite
## The command queue held by the crate's lane (`LaneState`, native/rust/src/lane/binding/
## queue.rs, spec 39 step 9c, Decision 3) as Godot reads it. Its rules are the crate's cargo
## tests (src/queue/tests.rs); these hold what only Godot sees: the fill checks as callables,
## a commit told on SimEvents with its command, and a commit's wave spawned before the lane's
## tick.

const NodeDef = preload("res://content/definitions/node_def.gd")
const WAVE := {"type": "wave", "units": [{"hp": 30, "dmg": 20}]}

var _lane: Object
var _slots := {"filled": 0}
var _heard: Array = []


func before_test() -> void:
	_slots = {"filled": 0}
	_heard = []
	_lane = NativeLibrary.made("LaneState")
	_lane.start(_nodes())
	_lane.respond({}, SimEvents)


func after_test() -> void:
	for connection in SimEvents.command_committed.get_connections():
		if connection["callable"].get_object() == self:
			SimEvents.command_committed.disconnect(connection["callable"])


## Four empty nodes: a wave marches on unopposed.
func _nodes() -> Array[NodeDef]:
	var nodes: Array[NodeDef] = []
	for index in 4:
		var node := NodeDef.new()
		node.node_type = NodeDef.NodeType.ORIGIN
		nodes.append(node)
	return nodes


func _is_filled() -> bool:
	return _slots["filled"] >= 1


## A check that reads the lane while it is lent.
func _no_player_units_yet() -> bool:
	return _lane.units_of("player") == 0


func _spawn_committed(command: Dictionary) -> void:
	_heard.append(command["type"])
	_lane.spawn_wave("player", command["units"], 0, 1)


func test_the_fill_check_is_the_callable_given() -> void:
	var id: int = _lane.enqueue(WAVE, _is_filled)

	assert_array(_lane.pending_commands()).contains([id])
	assert_bool(_lane.is_filled(id)).is_false()
	_slots["filled"] = 1
	assert_bool(_lane.is_filled(id)).is_true()


func test_a_cancelled_command_is_neither_pending_nor_filled() -> void:
	var id: int = _lane.enqueue(WAVE, _is_filled)
	_slots["filled"] = 1

	_lane.cancel(id)

	assert_array(_lane.pending_commands()).is_empty()
	assert_bool(_lane.is_filled(id)).is_false()


func test_an_under_filled_command_stays_and_is_not_told() -> void:
	var id: int = _lane.enqueue(WAVE, _is_filled)
	monitor_signals(SimEvents, false)

	_lane.on_tick_advanced(1)

	assert_array(_lane.pending_commands()).contains([id])
	await assert_signal(SimEvents).is_not_emitted("command_committed")


func test_a_filled_command_is_told_with_its_command_on_the_tick() -> void:
	_lane.enqueue(WAVE, _is_filled)
	_slots["filled"] = 1
	monitor_signals(SimEvents, false)

	_lane.on_tick_advanced(1)

	assert_array(_lane.pending_commands()).is_empty()
	await assert_signal(SimEvents).is_emitted("command_committed", WAVE)


func test_a_commit_s_wave_is_spawned_before_the_lane_s_tick_and_marches_in_it() -> void:
	SimEvents.command_committed.connect(_spawn_committed)
	_lane.enqueue(WAVE, _no_player_units_yet)

	_lane.on_tick_advanced(1)

	assert_array(_heard).is_equal(["wave"])
	var waves: Array = _lane.waves()
	assert_int(waves.size()).is_equal(1)
	assert_int(waves[0]["position"]).is_equal(1)
