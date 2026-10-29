class_name FormationCarry
extends RefCounted
## Bearing the wounded (Decision 126). A formation told to tend its downed - "recover" or
## "carry" - has a free unit beside each friendly downed body pick it up, while it isn't
## fighting and no foe is near the body: the nearest free unit that can still move under
## it. The body is load (wounds_body_weight a cell of it), slowing and tiring its bearer.
## "recover": the bearer sets off home with it alone (FormationStrays), and when it gets
## there both return to the reserve; "carry": the bearer keeps its place, bearing it on.
## A bearer that falls drops its body where it stands; a body that comes to is set down.
## A borne body still comes to, or dies of its wounds (FormationRecovery). The pass is the
## crate's (`carry/pass.rs`, spec 39 step 8b5-xxix): who picks up whom (`carry.rs`), the
## bodies following, dropped and coming home, the loads and the events. This mirrors the
## states, the carrying and the loads, and the squads' lists. Pure over the squads it is
## given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")

## The sheet's numbers the pass may change, which each unit keeps a copy of.
const MIRRORED: Array[StringName] = [
	&"carrying", &"carried_by", &"load_stage", &"fresh_speed", &"tiring", &"dodging"
]


## One tick: bodies follow their bearers, are dropped or set down, come home with them,
## and are picked up. `events` are this tick's so far (a bearer's "fled_home"). Returns
## [[bearer, its squad], ...] setting off home alone with a body.
static func step(squads: Array, tick: int, fight_seed: int, events: Array) -> Array:
	if squads.is_empty():
		return []
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var homes := PackedInt64Array()
	for event in events.filter(func(e): return e["type"] == "fled_home"):
		homes.append(event["unit"])
	var listed := FormationLocks.slots(squads)
	var at := PackedInt64Array([tick, fight_seed])
	var store: UnitStore = squads[0].store
	var field: Object = ColumnsLent.field(store.columns)
	var tunings := [tuning(), loads()]
	var did: PackedInt64Array = ColumnsLent.all(
		store.columns, func(): return field.carry_step(listed, homes, at, tunings[0], tunings[1])
	)
	var strays := _mirrored(squads, did, field)
	events.append_array(CrateEvents.taken(field, store))  # "fled_home", "borne"
	return strays


## Mirrors what the crate did, three numbers a unit: [what (0 changed, 1 home, 2 setting off
## home alone), its squad's index in the lists, its id]. Returns those setting off.
static func _mirrored(squads: Array, did: PackedInt64Array, field: Object) -> Array:
	var names: PackedStringArray = field.sheet_names()
	var strays := []
	for n in range(0, did.size(), 3):  # store: scattered (few: the bodies and their bearers)
		var squad: SkirmishSquad = squads[did[n + 1]]
		var unit: SkirmishUnit = squad.store.record_of(did[n + 2])
		if did[n] == 0:
			var state: int = squad.store.columns.value(unit.slot, UnitColumns.STATE)
			if state != unit.state:
				UnitState.put(squad, unit, state)
			var sheet: PackedFloat64Array = field.sheet(unit.slot)
			for name in MIRRORED:
				var value := sheet[names.find(name)]
				unit.set(name, value if typeof(unit.get(name)) == TYPE_FLOAT else int(value))
		elif did[n] == 1:
			squad.units.erase(unit)
			UnitState.regather(squad)
		else:
			strays.append([unit, squad])
	return strays


## What the pick-up takes of BattleTuning, in the order the crate reads it (CarryTuning).
static func tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			tuning.wounds_reach,
			tuning.wounds_guard_reach,
			tuning.wounds_body_weight,
			tuning.load_per_strength,
			tuning.load_easy,
			tuning.load_most,
			tuning.load_hauler,
		]
	)


## The pace, tiring and dodge by load stage, one list after another (LoadTuning).
static func loads() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(tuning.load_pace + tuning.load_tiring + tuning.load_dodge)
