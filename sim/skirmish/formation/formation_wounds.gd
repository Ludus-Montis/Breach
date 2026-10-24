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
## for one with "never_surrenders"). What the downed come to, and regeneration, are
## FormationRecovery's. Decided from one snapshot, then applied (Decision 97). The takings
## and the rolls are the crate's (`wounds.rs`, spec 39 step 6b): this hands it the squads'
## living and downed with what of each it doesn't hold, and lands what it decides - the
## states, the messengers spent, the morale shocks and the events. Pure over the squads it
## is given.

## What the crate says became of a body taken (wounds_tend).
enum Taken { CAPTURED, SENT_HOME, FINISHED }

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## This tick's takings of the downed. `interval` is ticks a second (a taker strikes on its
## melee interval).
static func tend(squads: Array, tick: int, interval: int, fight_seed: int, events: Array) -> void:
	if not _any_body(squads):
		return
	var counts := PackedInt32Array()
	var ints := PackedInt64Array()
	var reals := PackedFloat64Array()
	var listed := []  # per squad, [its living, its downed bodies], as handed in
	var codes := {}  # faction -> its code
	for squad in squads:
		var living: Array = squad.living()
		var bodies := _bodies(squad)
		listed.append([living, bodies])
		counts.append_array([living.size(), bodies.size()])
		for unit: SkirmishUnit in living + bodies:
			(
				ints
				. append_array(
					[
						unit.id,
						unit.slot,
						codes.get_or_add(unit.faction_id, codes.size()),
						_flags(unit),
						int(unit.traits.get("messenger", 0)),
						unit.dmg,
						int(unit.traits.get("cunning", 0)),
						int(unit.traits.get("thorough", 0)),
					]
				)
			)
			(
				reals
				. append_array(
					[
						unit.attributes.get("wits", UnitDef.AVERAGE),
						unit.attributes.get("constitution", UnitDef.AVERAGE),
						unit.melee_seconds,
					]
				)
			)
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var tuning := tuning()
	var taken: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.wounds_tend(counts, ints, reals, interval, fight_seed, tuning)
	)
	for at in range(0, taken.size(), 5):
		var squad: SkirmishSquad = squads[taken[at + 1]]
		var body: SkirmishUnit = listed[taken[at + 1]][1][taken[at + 2]]
		var other: SkirmishUnit = null
		if taken[at] != Taken.FINISHED:
			other = listed[taken[at + 3]][0][taken[at + 4]]
		_land(taken[at], [squad, body, other], squads, tick, events)


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


## The squad's downed bodies lying, not borne, in its order.
static func _bodies(squad: SkirmishSquad) -> Array:
	return UnitState.downed(squad).filter(
		func(unit): return unit.state == SkirmishUnit.State.DOWNED
	)


## A unit's flags as the crate takes them: 1 a captor, 2 it leads, 4 it plays dead.
static func _flags(unit: SkirmishUnit) -> int:
	var captor := int(unit.traits.get("captor", 0) > 0)
	return captor | int(unit.leadership > 0) << 1 | int(unit.playing_dead) << 2


## One body's taking lands: `taking` is [the body's squad, the body, its captor or the
## leader whose messenger sends it home (null when finished)].
static func _land(what: int, taking: Array, squads: Array, tick: int, events: Array) -> void:
	var squad: SkirmishSquad = taking[0]
	var body: SkirmishUnit = taking[1]
	var other: SkirmishUnit = taking[2]
	match what:
		Taken.CAPTURED:
			UnitState.put(squad, body, SkirmishUnit.State.TAKEN)
			var extra := {"by": other.id}
			events.append(FormationEvents.unit_event("captured", tick, squad, body, extra))
		Taken.SENT_HOME:
			other.traits["messenger"] = other.traits["messenger"] - 1
			UnitState.put(squad, body, SkirmishUnit.State.RELEASED)
			events.append(FormationEvents.unit_event("sent_home", tick, squad, body))
			for side in squads:
				if side.faction_id == body.faction_id and not side.is_destroyed():
					var shock := BattleTuning.current().wounds_messenger_shock
					FormationMorale.shock(side, shock, tick, events)
		Taken.FINISHED:
			UnitState.put(squad, body, SkirmishUnit.State.DEAD)
			events.append(FormationEvents.unit_event("finished", tick, squad, body))


## Routing units struck by this tick's `blows` ([striker, target, damage, ...]) that
## surrender.
static func surrender(
	blows: Array, squads: Array, fight_seed: int, tick: int, events: Array
) -> void:
	var routing := {}  # unit id -> its squad
	for squad in squads:
		if squad.state == SkirmishSquad.State.ROUTING:
			for unit in squad.living():
				routing[unit.id] = squad
	if routing.is_empty():
		return
	var caught := []
	var slots := PackedInt32Array()
	var ids := PackedInt64Array()
	var courages := PackedFloat64Array()
	for blow in blows:
		var unit: SkirmishUnit = blow[1]
		if not routing.has(unit.id) or blow[2] <= 0 or not unit.is_alive():
			continue
		if unit.traits.get("never_surrenders", 0) > 0:
			continue
		caught.append(unit)
		slots.append(unit.slot)
		ids.append(unit.id)
		courages.append(unit.courage)
	if caught.is_empty():
		return
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var chance := BattleTuning.current().wounds_surrender
	var gives: PackedByteArray = ColumnsLent.all(
		columns,
		func(): return field.wounds_surrenders(slots, ids, courages, tick, fight_seed, chance)
	)
	for index in caught.size():
		var unit: SkirmishUnit = caught[index]
		if gives[index] == 0 or not unit.is_alive():  # struck twice: it gave up once
			continue
		UnitState.put(routing[unit.id], unit, SkirmishUnit.State.TAKEN)
		events.append(FormationEvents.unit_event("surrendered", tick, routing[unit.id], unit))
		_retire(routing[unit.id], squads, tick, events)


## A squad left with no one standing is destroyed, freeing whoever fought it.
static func _retire(squad: SkirmishSquad, squads: Array, tick: int, events: Array) -> void:
	if squad.is_destroyed() and squad.state != SkirmishSquad.State.DESTROYED:
		squad.state = SkirmishSquad.State.DESTROYED
		FormationLocks.release(squad, squads)
		events.append(FormationEvents.squad_event("destroyed", tick, squad))
