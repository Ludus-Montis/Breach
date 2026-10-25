class_name FormationMorale
extends RefCounted
## Morale (Decision 82, spec 27 round 3): a formation's will to fight, 0 to 100. Its
## numbers are BattleTuning's (`morale_*`, content/tuning/battle_tuning.json).
## - **Ceiling:** its living units' mean courage, plus a step for each point of its best
##   living leader's leadership (Decision 81), at most 100. A fallen leader lowers it.
## - **Bands:** steady, shaken, wavering (strikes half again more slowly) and routing at
##   0. Short of a rout, a formation still meets its enemy at every band (Decision 101),
##   but the more shaken it is the softer its blows land: a share of their damage, by band.
## - **Shock** drains it: impact when struck on a side or the rear; a loss for each unit of
##   its own that falls, and per point of leadership when a leader falls.
## - **Pressure**, each second, by how many sides it fights on. A side with a friendly
##   squad near is supported and counts one less.
## - **Recovery**, each second out of contact: a little plus its leadership, up to the
##   ceiling.
## Pure over the squads it is given; the simulation calls it. Its pressure and recovery
## are the crate's (`morale.rs`); the ceiling, the bands and the shocks stay here, read and
## dealt by the passes that land what the crate decides.

enum Band { STEADY, SHAKEN, WAVERING, ROUTING }

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


static func ceiling(squad: SkirmishSquad) -> int:
	var living := squad.living()
	if living.is_empty():
		return 0
	var courage := 0
	for unit in living:
		courage += unit.courage
	return mini(
		100,
		roundi(float(courage) / living.size()) + _tuning().morale_per_leadership * leadership(squad)
	)


## The best living leader's leadership (Decision 81).
static func leadership(squad: SkirmishSquad) -> int:
	var best := 0
	for unit in squad.living():
		best = maxi(best, unit.leadership)
	return best


static func band(squad: SkirmishSquad) -> Band:
	var morale := _morale(squad)
	if morale >= _tuning().morale_steady_at:
		return Band.STEADY
	if morale >= _tuning().morale_shaken_at:
		return Band.SHAKEN
	return Band.WAVERING if morale > 0 else Band.ROUTING


## The attack interval for the units of a squad in `squad_band` (its `band`, read once a
## pass: Decision 144): half again as long while it wavers.
static func interval(squad_band: int, usual: int) -> int:
	return roundi(usual * 1.5) if squad_band == Band.WAVERING else usual


## Drains the squad's morale; a change of band is a "morale_band" event.
static func shock(squad: SkirmishSquad, amount: int, tick: int, events: Array) -> void:
	_set_morale(squad, _morale(squad) - amount, tick, events)


## The shock of this tick's dead in a squad: each loss, and more for each leader.
static func losses(squad: SkirmishSquad, fallen: Array, tick: int, events: Array) -> void:
	var amount := _tuning().morale_loss * fallen.size()
	for unit in fallen:
		if unit.leadership > 0:
			amount += _tuning().morale_leader_loss * unit.leadership
			var extra := {"unit": unit.id, "leadership": unit.leadership}
			events.append(FormationEvents.squad_event("leader_fell", tick, squad, extra))
	shock(squad, amount, tick, events)


## Once a second: pressure on squads fighting on several sides, recovery for those out of
## contact, and every squad held to its ceiling. The pressure and recovery are the crate's
## (`morale.rs`, spec 39 step 7a): this hands it each squad's morale, leadership and ceiling,
## and sets what it returns, in the lists' order.
static func step(squads: Array, tick: int, ticks_per_second: int, events: Array) -> void:
	if tick % maxi(1, ticks_per_second) != 0:
		return
	var stepped := []
	var stepping := PackedInt64Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if squad.is_destroyed() or squad.state == SkirmishSquad.State.ROUTING:
			continue
		stepped.append(squad)
		stepping.append_array([index, _morale(squad), leadership(squad), ceiling(squad)])
	if stepped.is_empty():
		return
	var tuning := _tuning()
	var rules := PackedFloat64Array(
		[tuning.morale_recovery, tuning.morale_support_cells, tuning.condition_cap]
	)
	rules.append_array(PackedFloat64Array(tuning.morale_pressure))
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var roster := CrateRoster.of(squads)
	var after: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.morale_step(roster, stepping, rules)
	)
	for index in stepped.size():
		_set_morale(stepped[index], after[index], tick, events)


static func _morale(squad: SkirmishSquad) -> int:
	if squad.morale < 0:
		squad.morale = ceiling(squad)
	return squad.morale


static func _set_morale(squad: SkirmishSquad, value: int, tick: int, events: Array) -> void:
	var before := band(squad)
	squad.morale = clampi(value, 0, 100)
	var after := band(squad)
	if after != before:
		var extra := {"band": Band.keys()[after], "morale": squad.morale}
		events.append(FormationEvents.squad_event("morale_band", tick, squad, extra))


static func _tuning() -> BattleTuning:
	return BattleTuning.current()
