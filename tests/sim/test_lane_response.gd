extends GdUnitTestSuite
## The kingdom's response held by the crate's lane (`LaneState.respond`,
## native/rust/src/lane/binding/response.rs, spec 39 step 9b-iii) as Godot reads it. Its rules
## are the crate's cargo tests (src/kingdom/tests.rs); these hold what only Godot sees: the
## tiers, the victory and the defeat emitted on the object given, a capture given to the
## economy held, a handler's spike sending its task force between arrivals, and a tier's
## dictionary staying its unit's truth.

const NodeDef = preload("res://content/definitions/node_def.gd")
const MOBILIZED := 3
const CORE := 3

var _lane: Object
var _hero_party: Dictionary
var _heard: Array = []


func before_test() -> void:
	_heard = []
	_hero_party = {"hp": 20, "dmg": 5}
	_lane = NativeLibrary.made("LaneState")
	_lane.start(_nodes())
	(
		_lane
		. respond(
			{
				"thresholds": [25, 50, 75, 90],
				"decay_per_tick": 30,
				"core": CORE,
				"spawn_index": CORE,
				"spawn_direction": -1,
				"units": {MOBILIZED: _hero_party},
			},
			SimEvents
		)
	)


func after_test() -> void:
	for connection in SimEvents.node_captured.get_connections():
		if connection["callable"].get_object() == self:
			SimEvents.node_captured.disconnect(connection["callable"])
	if SimEvents.victory.is_connected(_hear_victory):
		SimEvents.victory.disconnect(_hear_victory)


## Origin, a fort (10 hp, 3 dmg), the midpoint, the Core.
func _nodes() -> Array[NodeDef]:
	var nodes: Array[NodeDef] = []
	for index in 4:
		var node := NodeDef.new()
		node.node_type = NodeDef.NodeType.ORIGIN
		nodes.append(node)
	nodes[1].node_type = NodeDef.NodeType.FORT
	nodes[1].garrison = 1
	nodes[1].garrison_hp = 10
	nodes[1].garrison_dmg = 3
	return nodes


func _spike_on_taking_the_fort(node: int) -> void:
	if node == 1:
		_lane.add_suspicion(80)


func _hear_capture(node: int) -> void:
	_heard.append("captured %d" % node)


func _hear_victory() -> void:
	_heard.append("victory")


func test_a_change_of_tier_is_emitted_and_sends_its_task_force() -> void:
	monitor_signals(SimEvents, false)

	_lane.add_suspicion(80)

	assert_int(_lane.suspicion_tier()).is_equal(MOBILIZED)
	var dispatched: Array = _lane.dispatched()
	assert_int(dispatched.size()).is_equal(1)
	assert_str(dispatched[0]["owner"]).is_equal("defender")
	assert_int(dispatched[0]["position"]).is_equal(CORE)
	await assert_signal(SimEvents).is_emitted("suspicion_tier_changed", MOBILIZED)


func test_a_capture_on_the_lane_is_given_to_the_economy_held() -> void:
	_lane.spawn_wave("player", [{"hp": 30, "dmg": 20}], 0, 1)

	_lane.advance_positions(SimEvents)

	assert_bool(_lane.economy().is_awaiting_choice(1)).is_true()


func test_a_handler_s_spike_sends_a_task_force_a_later_wave_meets_that_tick() -> void:
	_lane.spawn_wave("player", [{"hp": 30, "dmg": 20}], 0, 1)
	_lane.spawn_wave("player", [{"hp": 30, "dmg": 4}], 2, 1)
	SimEvents.node_captured.connect(_spike_on_taking_the_fort)

	_lane.advance_positions(SimEvents)

	assert_int(_lane.moving_blockers()[0]["blocker"]["hp"]).is_equal(16)
	assert_int(_hero_party["hp"]).is_equal(16)


func test_taking_the_core_is_told_and_then_the_victory() -> void:
	SimEvents.node_captured.connect(_hear_capture)
	SimEvents.victory.connect(_hear_victory)
	_lane.spawn_wave("player", [{"hp": 30, "dmg": 20}], 2, 1)

	_lane.advance_positions(SimEvents)

	assert_array(_heard).is_equal(["captured 3", "victory"])


func test_a_tick_decays_the_tier_and_loses_the_task_force_it_destroyed() -> void:
	_lane.add_suspicion(80)
	_lane.spawn_wave("player", [{"hp": 30, "dmg": 20}], 1, 1)
	monitor_signals(SimEvents, false)

	_lane.on_tick_advanced(1)

	assert_int(_lane.suspicion_tier()).is_equal(2)
	assert_array(_lane.moving_blockers()).is_empty()
	assert_array(_lane.dispatched()).is_empty()
	await assert_signal(SimEvents).is_emitted("suspicion_tier_changed", 2)


func test_the_defeat_is_told_once_when_the_player_has_no_units() -> void:
	_lane.on_tick_advanced(1)
	monitor_signals(SimEvents, false)

	_lane.on_tick_advanced(2)

	await assert_signal(SimEvents).is_not_emitted("defeat")


func test_the_defeat_is_told_when_the_player_has_no_units() -> void:
	monitor_signals(SimEvents, false)

	_lane.on_tick_advanced(1)

	await assert_signal(SimEvents).is_emitted("defeat")
