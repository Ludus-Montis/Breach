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
## its pursuers tire, as well as at its leash. All numbers: BattleTuning. Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## The columns a tick's pass reads and writes, held for it (spec 35 step 6c, UnitView);
## each unit's condition is read alongside.
const STEPPED := [
	UnitColumns.STAMINA,
	UnitColumns.SPEED,
	UnitColumns.EXERTED,
	UnitColumns.BREATHER,
	UnitColumns.RUN_BUILD,
	UnitColumns.WAS_RUNNING,
]
## The columns a blow's cost writes.
const SPENT := [UnitColumns.STAMINA, UnitColumns.EXERTED, UnitColumns.BREATHER]


## Each unit's stamina spent on this tick's blows ([striker, ...]); `holder` holds the
## battle's store (its squads share it).
static func strike(holder, blows: Array) -> void:
	if blows.is_empty():
		return
	var view := UnitView.hold([holder], SPENT)  # a write per blow: held
	var tuning := BattleTuning.current()
	for blow in blows:
		_spend(view, blow[0], tuning.stamina_blow * blow[0].tiring)
	UnitView.release(view)


## One tick of `seconds`: runners and laden marchers tire, the rest recover after their
## breather, and each unit's pace follows its gait and how tired it is.
static func step(squads: Array, seconds: float) -> void:
	if squads.is_empty():
		return
	var tuning := BattleTuning.current()
	var held := UnitView.hold(squads, STEPPED)
	var conditions := {}  # store -> a read view of CONDITION (UnitView.read_once)
	for squad in squads:
		var view := UnitView.of(squad)
		var stamina: PackedFloat64Array = view.stamina
		var speeds: PackedFloat64Array = view.speed
		var exerted: PackedInt32Array = view.exerted
		var ran: PackedInt32Array = view.was_running
		var build: PackedFloat64Array = view.run_build
		var condition: PackedFloat64Array = (
			UnitView.read_once(conditions, squad, [UnitColumns.CONDITION]).condition
		)
		var running := runs(squad)
		var marching: bool = squad.state == SkirmishSquad.State.MOVING
		for unit in squad.living():
			var slot: int = unit.slot
			if unit.fresh_speed <= 0.0:
				unit.fresh_speed = speeds[slot]  # one made by hand, not from its sheet
			var gait := 1.0
			var runner: bool = (
				(running or squad.chasers.has(unit.id))
				and stage_at(stamina[slot], unit.max_stamina) < 2
			)
			if runner:
				var pursuing: bool = not squad.pursuit.is_empty() or squad.chasers.has(unit.id)
				gait = _build(view, unit, pursuing, seconds)
				if build[slot] > 0.0:
					_spend(view, unit, tuning.stamina_run * unit.tiring * seconds)
			elif marching and _march_cost(unit) > 0.0:
				_spend(view, unit, _march_cost(unit) * seconds)
			ran[slot] = 1 if runner else 0
			if exerted[slot] == 0:
				_breathe(view, unit, condition[slot], seconds)
			exerted[slot] = 0
			speeds[slot] = (
				unit.fresh_speed
				* gait
				* tuning.stamina_pace[stage_at(stamina[slot], unit.max_stamina)]
			)
	UnitView.release(held)


## True if the formation runs: fleeing; pursuing or retreating, unless its leader - or
## every one of its units - keeps to a march then ("pursues_at_march", "retreats_at_march");
## or hurried by the player - while it isn't fighting.
static func runs(squad: SkirmishSquad) -> bool:
	if squad.state == SkirmishSquad.State.ROUTING:
		return true
	if not squad.pursuit.is_empty() and not squad.pursuit["returning"]:
		return not _marches(squad, "pursues_at_march")
	if squad.state == SkirmishSquad.State.FIGHTING:
		return false
	if squad.order == SkirmishUnit.Order.RETREAT and not _marches(squad, "retreats_at_march"):
		return true
	return squad.hurry


## 0 fresh, 1 tired, 2 spent: the stage of a unit with `stamina` of `most`.
static func stage_at(stamina: float, most: float) -> int:
	var tuning := BattleTuning.current()
	var share := stamina / maxf(most, 0.000001)
	if share < tuning.stamina_spent:
		return 2
	return 1 if share < tuning.stamina_tired else 0


## The shift to its blows' margins its tiredness makes, with `stamina` left.
static func skill_shift(unit: SkirmishUnit, stamina: float) -> float:
	return BattleTuning.current().stamina_skill[stage_at(stamina, unit.max_stamina)]


## True if the unit is too tired to keep up a chase (a view holding STAMINA).
static func gives_up(view: UnitView, unit: SkirmishUnit) -> bool:
	return view.stamina[unit.slot] < unit.max_stamina * BattleTuning.current().stamina_give_up


## True if most of the squad's units are too tired to keep up a chase.
static func squad_gives_up(squad: SkirmishSquad) -> bool:
	var living := squad.living()
	if living.is_empty():
		return false
	var view := UnitView.read(squad, [UnitColumns.STAMINA])
	var tired := living.filter(func(u): return gives_up(view, u)).size()
	return tired * 2 > living.size()


## Its gait this tick of a run: building from its march to its run pace over
## run_build_seconds, a pursuer first reacting (the sooner, the quicker its wits).
static func _build(view: UnitView, unit: SkirmishUnit, pursuing: bool, seconds: float) -> float:
	var tuning := BattleTuning.current()
	var build: PackedFloat64Array = view.run_build
	var slot: int = unit.slot
	if view.was_running[slot] == 0:
		var wits: float = maxf(1.0, unit.attributes.get("wits", UnitDef.AVERAGE))
		build[slot] = -tuning.run_reaction * UnitDef.AVERAGE / wits if pursuing else 0.0
	build[slot] += seconds
	var built := clampf(build[slot] / maxf(tuning.run_build_seconds, 0.000001), 0.0, 1.0)
	return 1.0 + (unit.run_pace - 1.0) * built


## True if the squad keeps to a march for `trait_id`: its leader has it, or all its units.
static func _marches(squad: SkirmishSquad, trait_id: String) -> bool:
	var living := squad.living()
	if living.any(func(u): return u.leadership > 0 and u.traits.get(trait_id, 0) > 0):
		return true
	return not living.is_empty() and living.all(func(u): return u.traits.get(trait_id, 0) > 0)


## Spends stamina: it restarts the unit's breather (a view holding SPENT).
static func _spend(view: UnitView, unit: SkirmishUnit, amount: float) -> void:
	var stamina: PackedFloat64Array = view.stamina
	var exerted: PackedInt32Array = view.exerted
	var breather: PackedFloat64Array = view.breather
	stamina[unit.slot] = maxf(0.0, stamina[unit.slot] - amount)
	exerted[unit.slot] = 1
	breather[unit.slot] = 0.0


## A tick without cost: once its breather is long enough, it recovers, at its `condition`.
static func _breathe(view: UnitView, unit: SkirmishUnit, condition: float, seconds: float) -> void:
	var tuning := BattleTuning.current()
	var hardiness := (
		maxf(1.0, unit.attributes.get("constitution", UnitDef.AVERAGE)) / UnitDef.AVERAGE
	)
	if unit.on_face:
		return  # no breather on a climb face or in deep water (WalkModes)
	var breather: PackedFloat64Array = view.breather
	var stamina: PackedFloat64Array = view.stamina
	breather[unit.slot] += seconds
	if breather[unit.slot] < tuning.stamina_delay / hardiness:
		return
	var gain := (
		tuning.stamina_recovery * hardiness * clampf(condition, 0.0, tuning.condition_cap) * seconds
	)
	stamina[unit.slot] = minf(unit.max_stamina, stamina[unit.slot] + gain)


static func _march_cost(unit: SkirmishUnit) -> float:
	var costs: Array[float] = BattleTuning.current().load_march_tiring
	return costs[clampi(unit.load_stage, 0, costs.size() - 1)]
