extends GdUnitTestSuite
## Full integration replay of this slice's exact scripted path (specs/00), extended
## through to the victory event (test-first-order item 5 in
## specs/06-map-content-p-f-F-c.md). Composition wiring (connecting signals, marking
## a Task Force consumed when a combat_resolved event pertains to it specifically)
## mirrors what a future composition root (Phase 3 item 11) will do - none of these
## components self-subscribe to each other. Per Decision 16, reaching the Core wins
## unconditionally - defeating the Hero Party first is no longer a win prerequisite,
## just the realistic path a wave takes to get there without being wiped out first.

const SuspicionSystem = preload("res://sim/suspicion_system.gd")
const TaskForceDispatch = preload("res://sim/task_force_dispatch.gd")
const ScriptedBeatWatcher = preload("res://sim/scripted_beat_watcher.gd")
const NodeDef = preload("res://content/definitions/node_def.gd")

const P := 0
const FORT := 1
const MIDPOINT := 2
const CORE := 3


func _nodes() -> Array[NodeDef]:
	var node_p := NodeDef.new()
	node_p.node_type = NodeDef.NodeType.ORIGIN

	var node_fort := NodeDef.new()
	node_fort.node_type = NodeDef.NodeType.FORT
	node_fort.garrison = 1
	node_fort.garrison_hp = 10
	node_fort.garrison_dmg = 3

	var node_mid := NodeDef.new()
	node_mid.node_type = NodeDef.NodeType.ORIGIN

	var node_core := NodeDef.new()
	node_core.node_type = NodeDef.NodeType.ORIGIN

	var nodes: Array[NodeDef] = [node_p, node_fort, node_mid, node_core]
	return nodes


## A lane of the crate's (`LaneState`) over `_nodes()`.
func _lane() -> Object:
	var lane: Object = NativeLibrary.made("LaneState")
	lane.start(_nodes())
	return lane


func test_defeating_the_fort_then_the_hero_party_then_reaching_the_core_wins() -> void:
	var lane := _lane()
	var suspicion := SuspicionSystem.new([25, 50, 75, 90], 0)
	var dispatch := TaskForceDispatch.new(
		lane, CORE, -1, {SuspicionSystem.Tier.MOBILIZED: {"hp": 20, "dmg": 5}}
	)
	var watcher := ScriptedBeatWatcher.new(CORE)
	SimEvents.suspicion_tier_changed.connect(dispatch.on_tier_entered)
	SimEvents.node_captured.connect(watcher.on_node_captured)
	monitor_signals(SimEvents, false)

	# Beat 2: the player's horde destroys the fort outright (20 dmg > 10 hp) and
	# captures it with no casualties.
	lane.spawn_wave("player", [{"hp": 30, "dmg": 20}], P, 1)
	lane.advance_positions(SimEvents)
	assert_str(lane.node_state(FORT)["owner"]).is_equal("player")

	# Beat 3: attacking the fort raises suspicion via a detection spike, crossing
	# into Mobilized and dispatching the Hero Party from the Core.
	suspicion.add_suspicion(80)
	assert_int(dispatch.dispatched().size()).is_equal(1)

	# Beat 4/5: the horde and Hero Party meet at the midpoint and the horde wins
	# outright (20 dmg > 20 hp Hero Party) - a composition root would correlate this
	# combat_resolved event against TaskForceDispatch's own dispatched list; done
	# explicitly here.
	lane.advance_positions(SimEvents)
	var hero_party: Dictionary = dispatch.dispatched()[0]
	dispatch.mark_consumed(hero_party)
	assert_array(lane.moving_blockers()).is_empty()
	assert_array(lane.waves()[0]["units"]).is_not_empty()

	# Closing beat: the surviving horde reaches the Core and wins.
	lane.advance_positions(SimEvents)

	await assert_signal(SimEvents).is_emitted("victory")

	SimEvents.suspicion_tier_changed.disconnect(dispatch.on_tier_entered)
	SimEvents.node_captured.disconnect(watcher.on_node_captured)
