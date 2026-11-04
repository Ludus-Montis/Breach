extends GdUnitTestSuite

const TaskForceDispatch = preload("res://sim/task_force_dispatch.gd")
const NodeDef = preload("res://content/definitions/node_def.gd")
const SuspicionSystem = preload("res://sim/suspicion_system.gd")


func _nodes() -> Array[NodeDef]:
	var node := NodeDef.new()
	node.node_type = NodeDef.NodeType.ORIGIN
	var nodes: Array[NodeDef] = [node, node, node, node]
	return nodes


func _hero_party_stats() -> Dictionary:
	return {"hp": 70, "dmg": 18}


## A lane of the crate's (`LaneState`) over `_nodes()`.
func _lane() -> Object:
	var lane: Object = NativeLibrary.made("LaneState")
	lane.start(_nodes())
	return lane


func test_a_tier_with_no_assigned_unit_no_ops_safely() -> void:
	var lane := _lane()
	var dispatch := TaskForceDispatch.new(lane, 3, -1, {})  # no tier -> unit mapping at all

	dispatch.on_tier_entered(SuspicionSystem.Tier.WARY)

	assert_array(lane.moving_blockers()).is_empty()


func test_a_tier_with_an_assigned_unit_spawns_a_moving_blocker_on_the_lane() -> void:
	var lane := _lane()
	var dispatch := TaskForceDispatch.new(
		lane, 3, -1, {SuspicionSystem.Tier.MOBILIZED: _hero_party_stats()}
	)

	dispatch.on_tier_entered(SuspicionSystem.Tier.MOBILIZED)

	var movers: Array = lane.moving_blockers()
	assert_int(movers.size()).is_equal(1)
	assert_str(movers[0]["owner"]).is_equal("defender")
	assert_int(movers[0]["position"]).is_equal(3)
	assert_int(movers[0]["direction"]).is_equal(-1)
	assert_int(movers[0]["blocker"]["hp"]).is_equal(70)


func test_mark_completed_returns_a_task_force_to_the_roster() -> void:
	var lane := _lane()
	var dispatch := TaskForceDispatch.new(
		lane, 3, -1, {SuspicionSystem.Tier.MOBILIZED: _hero_party_stats()}
	)
	dispatch.on_tier_entered(SuspicionSystem.Tier.MOBILIZED)
	var task_force = dispatch.dispatched()[0]

	dispatch.mark_completed(task_force)

	assert_array(dispatch.dispatched()).is_empty()


func test_mark_consumed_removes_a_destroyed_task_force() -> void:
	var lane := _lane()
	var dispatch := TaskForceDispatch.new(
		lane, 3, -1, {SuspicionSystem.Tier.MOBILIZED: _hero_party_stats()}
	)
	dispatch.on_tier_entered(SuspicionSystem.Tier.MOBILIZED)
	var task_force = dispatch.dispatched()[0]

	dispatch.mark_consumed(task_force)

	assert_array(dispatch.dispatched()).is_empty()
