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
## crate's (`stamina.rs`, spec 39 step 6a): this hands it each squad's pursuit and hurry,
## which GDScript keeps, and what of each unit the crate doesn't hold.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## Each unit's stamina spent on this tick's blows ([striker, ...]); `holder` holds the
## battle's store (its squads share it).
static func strike(holder, blows: Array) -> void:
	if blows.is_empty():
		return
	var slots := PackedInt32Array()
	var tirings := PackedFloat64Array()
	for blow in blows:
		slots.append(blow[0].slot)
		tirings.append(blow[0].tiring)
	var columns: UnitColumns = holder.store.columns
	var field: Object = ColumnsLent.field(columns)
	var blow := BattleTuning.current().stamina_blow
	ColumnsLent.all(columns, func(): field.stamina_strike(slots, tirings, blow))


## One tick of `seconds`: runners and laden marchers tire, the rest recover after their
## breather, and each unit's pace follows its gait and how tired it is.
static func step(squads: Array, seconds: float) -> void:
	if squads.is_empty():
		return
	var columns: UnitColumns = squads[0].store.columns
	var listed := PackedInt64Array()
	var counts := PackedInt32Array()
	var ints := PackedInt64Array()
	var reals := PackedFloat64Array()
	for squad: SkirmishSquad in squads:
		var living: Array = squad.living()
		var pursuing := not squad.pursuit.is_empty()
		var returning: bool = pursuing and squad.pursuit["returning"]
		listed.append_array(
			[squad.slot, int(pursuing) | int(returning) << 1 | int(squad.hurry) << 2]
		)
		counts.append(living.size())
		for unit: SkirmishUnit in living:
			if unit.fresh_speed <= 0.0:  # one made by hand, not from its sheet
				# store: scattered (few: a unit made by hand, once)
				unit.fresh_speed = columns.value(unit.slot, UnitColumns.SPEED)
			ints.append_array([unit.id, unit.slot, unit.load_stage, _flags(unit)])
			(
				reals
				. append_array(
					[
						unit.fresh_speed,
						unit.max_stamina,
						unit.tiring,
						unit.run_pace,
						unit.attributes.get("wits", UnitDef.AVERAGE),
						unit.attributes.get("constitution", UnitDef.AVERAGE),
					]
				)
			)
	var field: Object = ColumnsLent.field(columns)
	var tuning := _tuning()
	ColumnsLent.all(
		columns, func(): field.stamina_step(listed, counts, ints, reals, seconds, tuning)
	)


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


## A unit's flags as the crate takes them: 1 it leads, 2 it pursues at a march, 4 it
## retreats at a march.
static func _flags(unit: SkirmishUnit) -> int:
	var leads := int(unit.leadership > 0)
	var pursues := int(unit.traits.get("pursues_at_march", 0) > 0)
	var retreats := int(unit.traits.get("retreats_at_march", 0) > 0)
	return leads | pursues << 1 | retreats << 2


## The tuning stamina_step takes.
static func _tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var out := PackedFloat64Array(
		[
			tuning.stamina_run,
			tuning.stamina_delay,
			tuning.stamina_recovery,
			tuning.run_build_seconds,
			tuning.run_reaction,
			tuning.condition_cap,
			tuning.stamina_tired,
			tuning.stamina_spent,
		]
	)
	out.append_array(tuning.stamina_pace)
	out.append_array(tuning.load_march_tiring)
	return out
