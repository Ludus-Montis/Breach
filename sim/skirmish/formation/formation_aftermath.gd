class_name FormationAftermath
extends RefCounted
## The fight and its aftermath as one crate call (`aftermath_step`, spec 39 step
## 8b5-xxxviii): the fight with its surrender (FormationMelee), the burial of the fallen
## (FormationDeaths), the takers sent (FormationTaking), the wounds' tending
## (FormationWounds) and what the downed come to (FormationRecovery), in that order over one
## roster, as FormationSimulation made them one call at a time. Then each pass's result is
## mirrored into GDScript's copies in the same order, and the events of all five taken.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationMelee = preload("res://sim/skirmish/formation/formation_melee.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")

## Numbers a unit or body takes in each part of what the crate returns.
const TAKEN := 2
const BURIED := 3
const TENDED := 5


## The fight and its aftermath over `squads` on `terrain`. `ticks` = [the ticks a second
## (a strike's interval and the tending's), the seed, the tick]; `seconds` the tick's;
## `rolls` whether blows roll. Returns the strays recovery set off alone, [[unit, its
## squad], ...], for FormationStrays.adopt.
static func step(
	squads: Array,
	ticks: Array,
	seconds: float,
	rolls: bool,
	terrain: FormationTerrain,
	events: Array
) -> Array:
	if squads.is_empty():
		return []
	var given := [CrateRoster.of(squads), UnitMover.ground(terrain)]  # each squad's living
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var made := args(ticks, seconds, rolls)
	var flat: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.callv(&"aftermath_step", [given[0]] + made + [given[1]])
	)
	var strays := mirror(squads, flat)
	events.append_array(CrateEvents.taken(field))  # the five passes' events, in order
	return strays


## The aftermath's arguments past the roster and before the ground (`aftermath_step`'s;
## the crate's `tick::given` builds the same for the whole tick): `ticks`, `seconds` and
## `rolls` as `step` takes them.
static func args(ticks: Array, seconds: float, rolls: bool) -> Array:
	var tuning := BattleTuning.current()
	var ints := PackedInt64Array(ticks + [int(rolls), tuning.wounds_messenger_shock])
	var floats := FormationMelee.rules()
	floats.append_array([seconds, tuning.take_hold, tuning.take_reach])
	return [ints, floats, BandTuning.rules(), BlowLanding.tuning(), FormationWounds.tuning()]


## GDScript's copies after the aftermath's `flat` result, each pass's in order; returns the
## strays recovery set off alone.
static func mirror(squads: Array, flat: PackedInt64Array) -> Array:
	var at := 1 + TAKEN * flat[0]
	_taken(squads, flat.slice(1, at))
	var end := at + 1 + BURIED * flat[at]
	FormationDeaths.mirror(squads, flat.slice(at + 1, end))
	at = end + 1 + TENDED * flat[end]
	FormationWounds.mirror(squads, flat.slice(end + 1, at))
	return FormationRecovery.mirror(squads, flat.slice(at))


## The units the surrender took, [its squad's listed index, its id] each: TAKEN.
static func _taken(squads: Array, taken: PackedInt64Array) -> void:
	for at in range(0, taken.size(), TAKEN):  # store: scattered (few: the tick's surrendered)
		var squad: SkirmishSquad = squads[taken[at]]
		UnitState.put(squad, squad.store.record_of(taken[at + 1]), SkirmishUnit.State.TAKEN)
