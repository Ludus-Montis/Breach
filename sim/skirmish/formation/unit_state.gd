class_name UnitState
extends RefCounted
## A unit's state (SkirmishUnit.State) and its one writer (spec 35 step 7a, Decision 147):
## every change to a unit's state goes through `put` (check_one_state.py holds that). Each
## squad keeps its living (standing: `is_alive()`) and its downed (DOWNED or CARRIED; taken
## and released units have left the field), both in `squad.units` order, so `living()` and
## `downed` cost nothing to ask; the passes that wanted the downed walked every unit.
##
## A write, and a join or a leaving (`regather`), mark the squad's lists for a rebuild at
## their next read: one filter of its units, however many changed. A rebuild after a join or
## a leaving also writes the squad's members to the squad store (`_list_members`). A rebuild
## makes new arrays, so a pass walking a list while it writes walks the list as it was when it
## began, as it did when every read filtered afresh. The lists handed out are read-only: a
## caller that wants to change one takes `.duplicate()`.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## A squad's kept lists (SkirmishSquad.kept).
class Kept:
	extends RefCounted
	var living: Array[SkirmishUnit] = []
	var downed: Array[SkirmishUnit] = []
	var stale := true
	var counted := 0  # how many units the squad had at the last rebuild
	var listed := false  # its members in the squad store are its units (spec 39 step 1)


## Sets the unit's state; `squad` is the one whose units it is in. The store's STATE column
## mirrors it, for the crate (spec 39 step 8b4-v). While CrateMirroring is on, the unit
## takes the column's state instead (the tick's last; spec 39 step 8b5-xlii).
static func put(squad, unit: SkirmishUnit, state: SkirmishUnit.State) -> void:
	squad.kept.stale = true
	if CrateMirroring.on:
		unit.state = squad.store.columns.value(unit.slot, UnitColumns.STATE)
		return
	unit.state = state
	squad.store.columns.put(unit.slot, UnitColumns.STATE, state)


## The unit was just given its slot in `store`: its STATE column takes its state (none
## while CrateMirroring is on: the crate's is the tick's).
static func admitted(store, unit: SkirmishUnit) -> void:
	if not CrateMirroring.on:
		store.columns.put(unit.slot, UnitColumns.STATE, unit.state)


## Sets the state of a unit no squad lists yet (one being made, or a test's lone body):
## the squad it joins gathers it (FormationCommand.enlist).
static func start(unit: SkirmishUnit, state: SkirmishUnit.State) -> void:
	unit.state = state


## The squad's units changed (a join, a leaving, a reordering): its lists are rebuilt at
## their next read. FormationCommand.enlist calls it for a join.
static func regather(squad) -> void:
	squad.kept.stale = true
	squad.kept.listed = false


## The squad's standing units, in its order (SkirmishSquad.living()).
static func living(squad) -> Array[SkirmishUnit]:
	return _kept(squad).living


## The squad's downed units, DOWNED or CARRIED, in its order.
static func downed(squad) -> Array[SkirmishUnit]:
	return _kept(squad).downed


static func _kept(squad) -> Kept:
	var kept: Kept = squad.kept
	var units: Array[SkirmishUnit] = squad.units
	if kept.stale:
		var living: Array[SkirmishUnit] = []
		var downed: Array[SkirmishUnit] = []
		for unit in units:
			var state := unit.state
			if state < SkirmishUnit.State.DEAD:
				living.append(unit)
			elif state == SkirmishUnit.State.DOWNED or state == SkirmishUnit.State.CARRIED:
				downed.append(unit)
		living.make_read_only()
		downed.make_read_only()
		kept.living = living
		kept.downed = downed
		kept.counted = units.size()
		kept.stale = false
		if not kept.listed:
			_list_members(squad)  # once a rebuild, however many joined or left
			kept.listed = true
	assert(kept.counted == units.size(), "squad.units changed without UnitState.regather")
	return kept


## Writes the squad's members - its units' unit-store slots, in its order - to the squad
## store (spec 39 step 1).
static func _list_members(squad) -> void:
	var units: Array[SkirmishUnit] = squad.units
	var slots := PackedInt32Array()
	slots.resize(units.size())
	for index in units.size():
		slots[index] = units[index].slot
	squad.store.squads.put_members(squad.slot, slots)
