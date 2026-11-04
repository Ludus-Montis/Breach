class_name TaskForceDispatch
extends RefCounted
## Spawns a Task Force (Messenger, Hero Party) onto the lane when a suspicion tier's
## entry action fires, per specs/04-suspicion-and-response.md and the parent spec's
## Core Roster and Task Force Dispatch section. Generalized engine, minimal content
## per Decision 5: a tier with no assigned unit in tier_to_unit_stats simply has
## nothing to dispatch.
##
## This slice has no Core Roster (finite unit pool) system to return a completed Task
## Force to or permanently remove a consumed one from - mark_completed/mark_consumed
## are kept as distinct methods per the spec's lifecycle language, but both currently
## just clear the active dispatch. The distinction becomes meaningful once a real
## roster exists to observe it.

var _lane: Object
var _spawn_index: int
var _spawn_direction: int
var _tier_to_unit_stats: Dictionary
var _dispatched: Array = []


func _init(
	lane: Object, spawn_index: int, spawn_direction: int, tier_to_unit_stats: Dictionary
) -> void:
	_lane = lane
	_spawn_index = spawn_index
	_spawn_direction = spawn_direction
	_tier_to_unit_stats = tier_to_unit_stats


func dispatched() -> Array:
	return _dispatched


func on_tier_entered(tier: int) -> void:
	if not _tier_to_unit_stats.has(tier):
		return
	var blocker: Dictionary = _tier_to_unit_stats[tier]
	var task_force: Dictionary = _lane.spawn_moving_blocker(
		"defender", blocker, _spawn_index, _spawn_direction
	)
	_dispatched.append(task_force)


func mark_completed(task_force: Dictionary) -> void:
	_dispatched.erase(task_force)


func mark_consumed(task_force: Dictionary) -> void:
	_dispatched.erase(task_force)
