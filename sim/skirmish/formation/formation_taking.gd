class_name FormationTaking
extends RefCounted
## Taking the downed (spec 30 round 3, part 8; Decisions 121 and 124: "units must either
## kill or capture"). A group out of a fight sends its free units - in their places, not
## loose or fleeing - out to the downed foes it can see within take_reach of it, one unit to
## a body, nearest first; each walks to its body (FormationWalk.target_of) and, beside it,
## takes it by the wounds rules (FormationWounds: finish, capture, messenger, playing dead).
## The group holds while its takers are out, up to take_hold seconds - for good with "no
## man left behind" - and under "fall behind, left behind", or when its command leaves the
## downed, it presses on and takes none. A taker comes back to its place once its body is
## dead, taken, borne away or come to, or the hold runs out. Pairs go by distance, then the
## units' and bodies' seeded draws, never by ids or lists (Decision 97). A group that is
## threatened - it sees a standing foe (FormationPerception, Decision 134) - fights first:
## it sends no takers, calls back those out and doesn't hold for them. The whole of it is
## the crate's (`taking/pass.rs`, spec 39 step 8b5-xxiii): the recall, the hold, who goes
## out to which body and "sent_to_take"; the squads' takings ({taker's id: its body's store
## slot}), the hold's start and where each gave up are held there, read through the squad.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## One tick: takers whose work is done come back, then groups send free units out - none
## from a group threatened as the tick's perception looked (FormationPerception.see; the
## crate keeps it, SkirmishSquad.threatened, spec 39 step 8b5-xxxvii), whose takers all
## come back.
static func step(squads: Array, tick: int, seconds: float, fight_seed: int, events: Array):
	if squads.is_empty():
		return
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := PackedFloat64Array([_tuning().take_hold, _tuning().take_reach])
	ColumnsLent.all(
		columns, func(): return field.taking_step(listed, tick, seconds, fight_seed, tuning)
	)
	events.append_array(CrateEvents.taken(field))  # "sent_to_take"


## True while the squad holds for its takers.
static func holding(squad: SkirmishSquad) -> bool:
	return not squad.taking.is_empty()


static func _tuning() -> BattleTuning:
	return BattleTuning.current()
