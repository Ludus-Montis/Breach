class_name WaveFormPanel
extends Control
## Player input for forming a marching wave, per specs/08-composition-root-and-input.md.
## Grem are bought instantly (no "in production" state exists in this slice), so
## "filled" for this map's wave command just means "at least one unit was queued" -
## the CommandQueue latch (Decision 3) still applies, delaying the actual march to the
## next tick boundary, not the buying.
##
## max_pending_units (found necessary via manual playtest - queueing was unbounded)
## is a temporary stopgap for the formation-grid system (Phase 4 item 6, deferred
## pending its own design pass), using the same "3 slots" starting shape the plan
## already commits to rather than a number invented here.

var economy: Object
## The lane (`LaneState`), which holds the command queue (spec 39 step 9c).
var command_queue: Object
var max_pending_units: int = 3

var _pending_units: Array = []


func on_queue_grem_pressed(unit_def: UnitDef) -> bool:
	if _pending_units.size() >= max_pending_units:
		return false
	if not economy.spend("food", unit_def.cost_food):
		return false
	_pending_units.append({"hp": unit_def.hp, "dmg": unit_def.melee_damage()})
	return true


func pending_count() -> int:
	return _pending_units.size()


func on_march_pressed(start_index: int, direction: int) -> void:
	if _pending_units.is_empty():
		return
	var command := {
		"type": "wave",
		"owner": "player",
		"units": _pending_units.duplicate(),
		"start_index": start_index,
		"direction": direction,
	}
	command_queue.enqueue(command, func(): return true)
	_pending_units.clear()
