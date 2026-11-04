class_name FormationDeaths
extends RefCounted
## The fallen (specs/22-formation-feel-test.md, Decisions 47, 82 and 121): a unit brought to
## 0 hp is downed - its hp stops at 0, the excess lost, and it lies where it fell - unless
## the blows took it past minus its constitution, which kills it outright. Either way it
## leaves the fight and shakes its squad, the ranks behind step up (units find the fight
## themselves: Decision 88), and a squad with no one left is destroyed, freeing whoever
## fought it. What becomes of the downed is FormationWounds'. The fallen are those on the
## store's fell queue (UnitHealth, Decision 145), so deaths costs per fallen unit, not per
## army, each buried with the squad it is in now, the squads in the lists' order and each
## one's fallen in its own. The burial is the crate's (`deaths/bury.rs`, spec 39 step
## 8b5-xxii): the fates, states and wounds, the morale, the step-ups, the destroyed and their
## events; this mirrors GDScript's copies of the units' states and wounds, and takes the
## events. Pure over the squads.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


static func bury(squads: Array, tick: int, events: Array) -> void:
	if squads.is_empty():
		return
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var buried: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.deaths_bury(listed, BandTuning.rules(), tick)
	)
	mirror(squads, buried)
	events.append_array(CrateEvents.taken(field))  # "died", "downed", ..., "destroyed"


## GDScript's copies of the units `deaths_bury` buried (three numbers a unit: its squad's
## listed index, its id, the wounds it bears now or -1 if it died).
static func mirror(squads: Array, buried: PackedInt64Array) -> void:
	for at in range(0, buried.size(), 3):  # store: scattered (few: the tick's fallen)
		var squad: SkirmishSquad = squads[buried[at]]
		var unit: SkirmishUnit = squad.store.record_of(buried[at + 1])
		if buried[at + 2] < 0:
			UnitState.put(squad, unit, SkirmishUnit.State.DEAD)
		else:
			UnitState.put(squad, unit, SkirmishUnit.State.DOWNED)
			unit.wounded = buried[at + 2]  # the sheet's already; GDScript's copy follows
