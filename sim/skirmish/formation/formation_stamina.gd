class_name FormationStamina
extends RefCounted
## Gaits and stamina (spec 28 part 7, Decisions 125 and 126). A unit marches at its speed
## or runs at its speed times its run pace, building up to it over a moment; a pursuer
## first reacts, giving a retreat its head start. A formation runs while it flees, pursues
## or retreats - unless its leader, or all its units, keep to a march then - or when the
## player hurries it; a unit chasing on its own runs; a spent unit can't run, so a
## formation of them walks. Marching costs nothing unless its load makes it (load_march_tiring);
## running costs stamina_run a second and each blow stamina_blow, faster the heavier the
## load. Each cost restarts a breather: once stamina_delay seconds pass with none, stamina
## is regained at stamina_recovery a second - the delay shorter and the rate faster the
## hardier the unit (its constitution), the rate times its condition. Below stamina_tired
## a unit is tired, below stamina_spent spent: slower and less skilled; a pursuit ends when
## its pursuers tire, as well as at its leash. All numbers: BattleTuning. The rules are the
## crate's (`stamina.rs`, spec 39 step 6a), and so is the pass (step 8b5-xxxi).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")


## One tick of `seconds`: runners and laden marchers tire, the rest recover after their
## breather, and each unit's pace follows its gait and how tired it is. The pass is the
## crate's (`stamina/pass.rs`, spec 39 step 8b5-xxxi): each squad's pursuit and hurry and
## each unit's numbers, traits and attributes are read there.
static func step(squads: Array, seconds: float) -> void:
	if squads.is_empty():
		return
	for squad: SkirmishSquad in squads:
		squad.living()  # its members written to the squad store, if its lists changed
	var store: UnitStore = squads[0].store
	var field: Object = ColumnsLent.field(store.columns)
	var listed := FormationLocks.slots(squads)
	var rules := tuning()
	var made: PackedInt64Array = ColumnsLent.all(
		store.columns, func(): return field.stamina_step(listed, seconds, rules)
	)
	mirror(store, made)


## The units made by hand that took their speed as fresh in the crate (their ids, as
## `stamina_step` returns them): GDScript's copy of the fresh speed set from the sheet.
static func mirror(store: UnitStore, made: PackedInt64Array) -> void:
	if made.is_empty():
		return
	var field: Object = ColumnsLent.field(store.columns)
	var fresh: int = field.sheet_names().find("fresh_speed")
	for id in made:  # store: scattered (few: a unit made by hand, once)
		var unit: SkirmishUnit = store.record_of(id)
		unit.fresh_speed = field.sheet(unit.slot)[fresh]  # its speed, given it as fresh


## 0 fresh, 1 tired, 2 spent: the stage of a unit with `stamina` of `most`.
static func stage_at(stamina: float, most: float) -> int:
	var tuning := BattleTuning.current()
	return CrateScrum.core().stamina_stage(
		stamina, most, tuning.stamina_tired, tuning.stamina_spent
	)


## The shift to its blows' margins its tiredness makes, with `stamina` left.
static func skill_shift(unit: SkirmishUnit, stamina: float) -> float:
	return BattleTuning.current().stamina_skill[stage_at(stamina, unit.max_stamina)]


## True if most of the squad's units are too tired to keep up a chase.
static func squad_gives_up(squad: SkirmishSquad) -> bool:
	var slots := PackedInt32Array()
	var most := PackedFloat64Array()
	for unit: SkirmishUnit in squad.living():
		slots.append(unit.slot)
		most.append(unit.max_stamina)
	var columns: UnitColumns = squad.store.columns
	var give_up := BattleTuning.current().stamina_give_up
	return ColumnsLent.all(
		columns, func(): return ColumnsLent.field(columns).stamina_gives_up(slots, most, give_up)
	)


## The tuning stamina_step takes.
static func tuning() -> PackedFloat64Array:
	var battle := BattleTuning.current()
	var out := PackedFloat64Array(
		[
			battle.stamina_run,
			battle.stamina_delay,
			battle.stamina_recovery,
			battle.run_build_seconds,
			battle.run_reaction,
			battle.condition_cap,
			battle.stamina_tired,
			battle.stamina_spent,
		]
	)
	out.append_array(battle.stamina_pace)
	out.append_array(battle.load_march_tiring)
	return out
