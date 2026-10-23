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
## each unit's condition, and whether it is on a climb face or in deep water (ON_FACE, step
## 7c), are read alongside (READ).
const STEPPED := [
	UnitColumns.STAMINA,
	UnitColumns.SPEED,
	UnitColumns.EXERTED,
	UnitColumns.BREATHER,
	UnitColumns.RUN_BUILD,
	UnitColumns.WAS_RUNNING,
]
const READ := [UnitColumns.CONDITION, UnitColumns.ON_FACE]
## The columns a blow's cost writes.
const SPENT := [UnitColumns.STAMINA, UnitColumns.EXERTED, UnitColumns.BREATHER]


## Each unit's stamina spent on this tick's blows ([striker, ...]); `holder` holds the
## battle's store (its squads share it).
static func strike(holder, blows: Array) -> void:
	if blows.is_empty():
		return
	var view := UnitView.hold([holder], SPENT)  # a write per blow: held
	var spent := [view.stamina, view.exerted, view.breather]
	var tuning := BattleTuning.current()
	for blow in blows:
		_spend(spent[0], spent[1], spent[2], blow[0].slot, tuning.stamina_blow * blow[0].tiring)
	UnitView.release(view)


## One tick of `seconds`: runners and laden marchers tire, the rest recover after their
## breather, and each unit's pace follows its gait and how tired it is.
static func step(squads: Array, seconds: float) -> void:
	if squads.is_empty():
		return
	var held := UnitView.hold(squads, STEPPED)
	var reads := {}  # store -> a read view of READ (UnitView.read_once)
	for squad in squads:
		var read := UnitView.read_once(reads, squad, READ)
		_step_squad(squad, UnitView.of(squad), [read.condition, read.on_face], seconds)
	UnitView.release(held)


## `step` for one squad: its view's columns taken once into typed locals and indexed per
## unit (a getter per unit would be a call each). `read` is [CONDITION, ON_FACE].
static func _step_squad(squad: SkirmishSquad, view: UnitView, read: Array, seconds: float) -> void:
	var tuning := BattleTuning.current()
	var stamina: PackedFloat64Array = view.stamina
	var speeds: PackedFloat64Array = view.speed
	var exerted: PackedInt32Array = view.exerted
	var breather: PackedFloat64Array = view.breather
	var ran: PackedInt32Array = view.was_running
	var build: PackedFloat64Array = view.run_build
	var condition: PackedFloat64Array = read[0]
	var on_face: PackedInt32Array = read[1]
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
			gait = _build(build, ran[slot] != 0, unit, pursuing, seconds)
			if build[slot] > 0.0:
				_spend(stamina, exerted, breather, slot, tuning.stamina_run * unit.tiring * seconds)
		elif marching and _march_cost(unit) > 0.0:
			_spend(stamina, exerted, breather, slot, _march_cost(unit) * seconds)
		ran[slot] = 1 if runner else 0
		if exerted[slot] == 0 and on_face[slot] == 0:  # no breather on a face (the walk modes)
			_breathe(stamina, breather, unit, condition[slot], seconds)
		exerted[slot] = 0
		speeds[slot] = (
			unit.fresh_speed * gait * tuning.stamina_pace[stage_at(stamina[slot], unit.max_stamina)]
		)


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
	var stamina: PackedFloat64Array = UnitView.read(squad, [UnitColumns.STAMINA]).stamina
	var give_up := BattleTuning.current().stamina_give_up
	var tired := living.filter(func(u): return stamina[u.slot] < u.max_stamina * give_up).size()
	return tired * 2 > living.size()


## Its gait this tick of a run: building from its march to its run pace over
## run_build_seconds, a pursuer first reacting (the sooner, the quicker its wits). `build`
## is the RUN_BUILD column, written in place; `was_running` whether it ran last tick.
static func _build(
	build: PackedFloat64Array, was_running: bool, unit: SkirmishUnit, pursuing: bool, seconds: float
) -> float:
	var tuning := BattleTuning.current()
	var slot: int = unit.slot
	if not was_running:
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


## Spends `amount` of the slot's stamina: it restarts its breather. The columns are
## STAMINA, EXERTED and BREATHER, written in place.
static func _spend(
	stamina: PackedFloat64Array,
	exerted: PackedInt32Array,
	breather: PackedFloat64Array,
	slot: int,
	amount: float
) -> void:
	stamina[slot] = maxf(0.0, stamina[slot] - amount)
	exerted[slot] = 1
	breather[slot] = 0.0


## A tick without cost: once its breather is long enough, it recovers, at its `condition`.
## `stamina` and `breather` are its columns, written in place.
static func _breathe(
	stamina: PackedFloat64Array,
	breather: PackedFloat64Array,
	unit: SkirmishUnit,
	condition: float,
	seconds: float
) -> void:
	var tuning := BattleTuning.current()
	var hardiness := (
		maxf(1.0, unit.attributes.get("constitution", UnitDef.AVERAGE)) / UnitDef.AVERAGE
	)
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
