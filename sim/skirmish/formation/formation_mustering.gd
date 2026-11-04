class_name FormationMustering
extends RefCounted
## The strays through the groups as one crate call (`mustering_step`, spec 39 step
## 8b5-xxxix): the strays recovery set off adopted (FormationStrays), morale's second
## (FormationMorale), the rout (FormationRout), carrying (FormationCarry), the bearers
## carrying set off adopted in turn, and the groups splitting and forming up
## (FormationGroups), in that order, as FormationSimulation made them one call at a time. The
## squads the crate lists grow as GDScript's did: each squad a pass opens is listed after the
## rest for the passes after it. Then each pass's result is mirrored into GDScript's copies
## in the same order, and the events of all of them taken.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")

## Numbers each part of what the crate returns takes a stray, a router and a carried unit.
const ADOPTED := 2
const ROUTED := 3
const CARRIED := 3


## The passes over `squads` on `terrain`, with recovery's `strays` ([[unit, its squad], ...]).
## `at` = [tick, the ticks a second, seed, the next free squad id]; `pace` = [cells a second
## at speed 1, the tick's seconds]. New squads are added to `squads`, those taken in leave
## it; returns the next free squad id.
static func step(
	squads: Array, strays: Array, at: Array, pace: Array, terrain: FormationTerrain, events: Array
) -> int:
	var next_id: int = at[3]
	if squads.is_empty():
		return next_id
	var store: UnitStore = squads[0].store
	var field: Object = store.field()
	var flat := PackedInt32Array()
	for stray in strays:
		flat.append_array([stray[1].slot, stray[0].slot])
	field.put_squads_issued(next_id - 1)
	var roster := CrateRoster.of(squads, terrain)  # each squad's living, fresh
	var given := [roster, flat] + args(at, pace) + [UnitMover.ground(terrain)]
	var out: PackedInt64Array = ColumnsLent.all(
		store.columns, func(): return field.callv(&"mustering_step", given)
	)
	next_id = mirror(squads, strays, out, next_id)
	events.append_array(CrateEvents.taken(field, store))  # the passes' events, in order
	return next_id


## The mustering's arguments past the roster and its strays and before the ground
## (`mustering_step`'s, and the whole tick's part, `tick_step`): `at` and `pace` as `step`
## takes them.
static func args(at: Array, pace: Array) -> Array:
	var ints := PackedInt64Array([at[0], at[2], int(at[0] % maxi(1, at[1]) == 0)])
	var tunings := [
		_morale(),
		FormationRout.rules(pace[0] * pace[1], pace[1]),
		FormationCarry.tuning(),
		FormationCarry.loads(),
		FormationGroups.rules(),
	]
	return [ints, tunings, BandTuning.rules()]


## GDScript's copies after the mustering's `out` result, each pass's in order, with
## recovery's `strays` and the squads numbered from `next_id`; returns the next free id.
static func mirror(squads: Array, strays: Array, out: PackedInt64Array, next_id: int) -> int:
	var field: Object = squads[0].store.field()
	var end := 1 + ADOPTED * out[0]
	next_id = FormationStrays.mirror(strays, squads, out.slice(1, end), next_id)
	var at_rout := end + 1 + ROUTED * out[end]
	FormationRout.mirror(squads, out.slice(end + 1, at_rout))
	end = at_rout + 1 + CARRIED * out[at_rout]
	var bearers := FormationCarry.mirror(squads, out.slice(at_rout + 1, end), field)
	var at_groups := end + 1 + ADOPTED * out[end]
	next_id = FormationStrays.mirror(bearers, squads, out.slice(end + 1, at_groups), next_id)
	return FormationGroups.mirror(out.slice(at_groups), squads, next_id)


## Morale's tuning as FormationMorale.step hands it the crate: [morale_recovery,
## morale_support_cells, condition_cap, the morale_pressure...].
static func _morale() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[tuning.morale_recovery, tuning.morale_support_cells, tuning.condition_cap]
	)
	rules.append_array(PackedFloat64Array(tuning.morale_pressure))
	return rules
