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
##   its own that falls, and per point of leadership when a leader falls (the burial's, in
##   the crate: `deaths/bury.rs`, spec 39 step 8b5-xxii).
## - **Pressure**, each second, by how many sides it fights on. A side with a friendly
##   squad near is supported and counts one less.
## - **Recovery**, each second out of contact: a little plus its leadership, up to the
##   ceiling.
## Pure over the squads it is given; the simulation calls it. Its pressure and recovery
## are the crate's (`morale.rs`), and its ceiling, bands and shocks (`bands.rs`, spec 39 step
## 8a2-iv): this hands it the living's courage and leadership and each squad's morale, keeps
## each squad's morale, and makes the events.

enum Band { STEADY, SHAKEN, WAVERING, ROUTING }

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


static func ceiling(squad: SkirmishSquad) -> int:
	var courage := PackedInt64Array()
	var leaderships := PackedInt64Array()
	for unit in squad.living():
		courage.append(unit.courage)
		leaderships.append(unit.leadership)
	return CrateScrum.core().morale_ceiling(courage, leaderships, _rules())


## The best living leader's leadership (Decision 81).
static func leadership(squad: SkirmishSquad) -> int:
	var leaderships := PackedInt64Array()
	for unit in squad.living():
		leaderships.append(unit.leadership)
	return CrateScrum.core().morale_leadership(leaderships)


static func band(squad: SkirmishSquad) -> Band:
	return CrateScrum.core().morale_band(_morale(squad), _rules()) as Band


## The attack interval for the units of a squad in `squad_band` (its `band`, read once a
## pass: Decision 144): half again as long while it wavers.
static func interval(squad_band: int, usual: int) -> int:
	return CrateScrum.core().morale_interval(squad_band, usual)


## Drains the squad's morale; a change of band is a "morale_band" event.
static func shock(squad: SkirmishSquad, amount: int, tick: int, events: Array) -> void:
	_set_morale(squad, _morale(squad) - amount, tick, events)


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
	squad.morale = CrateScrum.core().morale_held(value)
	var after := band(squad)
	if after != before:
		var extra := {"band": Band.keys()[after], "morale": squad.morale}
		events.append(FormationEvents.squad_event("morale_band", tick, squad, extra))


static func _tuning() -> BattleTuning:
	return BattleTuning.current()


## The tuning the crate's rules read (BandTuning).
static func _rules() -> PackedInt64Array:
	return BandTuning.rules()
