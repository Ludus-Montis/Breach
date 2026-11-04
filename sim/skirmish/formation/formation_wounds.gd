class_name FormationWounds
extends RefCounted
## What becomes of the downed and the caught (Decision 121, spec 28 part 6). Units don't
## leave the downed: once no friend of a downed unit stands near it (wounds_guard_reach)
## and a standing foe beside it (wounds_reach) has no foe of its own near, that foe takes
## it - captures it if it is a captor, else strikes it on its blows' interval, down into
## death's door, constitution deep, till it dies. The one exception: a foe whose
## formation's leader can send a messenger (its "messenger" trait, one a level) lets it go,
## to carry word home - a morale blow to every standing formation of its side. A routing
## unit struck may surrender in place, by a seeded chance from its want of courage (none
## for one with "never_surrenders"): the fight's crate pass (`fight/surrender.rs`, spec 39
## step 8b5-xx). What the downed come to, and regeneration, are FormationRecovery's. Decided
## from one snapshot, then applied (Decision 97). The whole of it is the crate's
## (`wounds/pass.rs`, spec 39 step 8b5-xxiv): the takings and their rolls, the states, the
## messengers spent, the morale shocks and the events; this mirrors the states and the
## messengers on GDScript's copies. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## This tick's takings of the downed. `interval` is ticks a second (a taker strikes on its
## melee interval).
static func tend(squads: Array, tick: int, interval: int, fight_seed: int, events: Array) -> void:
	if not _any_body(squads):
		return
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var listed := FormationLocks.slots(squads)
	var at := PackedInt64Array([tick, interval, fight_seed, _tuning().wounds_messenger_shock])
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := tuning()
	var landed: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.wounds_step(listed, at, tuning, BandTuning.rules())
	)
	mirror(squads, landed)
	events.append_array(CrateEvents.taken(field))  # "captured", "sent_home", "finished", ...


## GDScript's copies of the bodies `wounds_step` took (five numbers a body: its squad's
## listed index, its id, its state now, and for one sent home its leader's listed index and
## id, else -1, -1): each state, and a sending leader's messengers.
static func mirror(squads: Array, landed: PackedInt64Array) -> void:
	for n in range(0, landed.size(), 5):  # store: scattered (few: the tick's takings)
		var squad: SkirmishSquad = squads[landed[n]]
		var body: SkirmishUnit = squad.store.record_of(landed[n + 1])
		UnitState.put(squad, body, landed[n + 2])
		if landed[n + 3] >= 0:  # sent home: its leader has one messenger fewer
			var leader: SkirmishUnit = squads[landed[n + 3]].store.record_of(landed[n + 4])
			leader.set_trait("messenger", leader.traits["messenger"] - 1)


## What the downed's passes take of BattleTuning, in the order the crate reads it
## (WoundsTuning).
static func tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			tuning.wounds_reach,
			tuning.wounds_guard_reach,
			tuning.wounds_condition,
			tuning.wounds_danger_reach,
			tuning.wounds_play_dead,
			tuning.wounds_cunning,
			tuning.wounds_play_dead_check,
			tuning.wounds_trait_wits,
			tuning.wounds_believe_die,
			tuning.wounds_wake_seconds,
			tuning.wounds_wake_spread[0],
			tuning.wounds_wake_spread[1],
			tuning.condition_cap,
			tuning.condition_drain_below,
			tuning.condition_drain,
		]
	)


## True if a squad has a downed body lying (a borne one isn't taken).
static func _any_body(squads: Array) -> bool:
	for squad in squads:
		for unit in UnitState.downed(squad):
			if unit.state == SkirmishUnit.State.DOWNED:
				return true
	return false


static func _tuning() -> BattleTuning:
	return BattleTuning.current()
