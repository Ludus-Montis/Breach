class_name FormationOpening
extends RefCounted
## The tick's opening as one crate call (`opening.rs`, spec 39 step 8b5-xxxv): the orders
## queued apply (FormationOrders), each squad sees (FormationPerception), the fronts and then
## the edges lock (FormationFronts, FormationEdges), the groups find their way
## (FormationPathing, on ground only) and the squads move and join (the move's pass and
## FormationJoins), all over one roster, every pass's events in the crate's log in that order.
## The crate holds each squad's route and its code (steps 8b5-xxxvi and 8b5-xxxvii), which
## GDScript's route reads (SquadRoutes): a pursuer that took its quarry's route and a group
## that re-routed read theirs with no copy made here. What each squad sees becomes the tick's
## perception (the crate keeps each one's threat too) and the joined squads leave the list.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationOrders = preload("res://sim/skirmish/formation/formation_orders.gd")
const FormationPathing = preload("res://sim/skirmish/formation/formation_pathing.gd")
const FormationJoins = preload("res://sim/skirmish/formation/formation_joins.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")


## One tick's opening over `squads` on `terrain` (null: open ground), `ticks` [tick, seed],
## `moving` [TRAVEL_SCALE, seconds a tick, the lane's length]; adds its events. Returns the
## tick's perception (FormationPerception.see's).
static func step(
	squads: Array, terrain: FormationTerrain, ticks: Array, moving: Array, events: Array
) -> Dictionary:
	if squads.is_empty():
		return {}
	var field: Object = squads[0].store.field()
	var given := [CrateRoster.of(squads), UnitMover.ground(terrain)]
	var flat: PackedInt64Array = field.callv(&"opening_step", given + args(ticks, moving))
	events.append_array(CrateEvents.taken(field))
	return mirror(squads, flat)


## The opening's arguments past the roster and the ground (`opening_step`'s; the crate's
## `tick::given` builds the same for the whole tick): `ticks` [tick, seed], `moving` as
## `step` takes it.
static func args(ticks: Array, moving: Array) -> Array:
	var tuning := BattleTuning.current()
	return [
		PackedInt64Array(ticks),
		FormationOrders.rules(),
		BandTuning.rules(),
		PackedInt64Array([tuning.morale_side_impact, tuning.morale_rear_impact]),
		FormationPathing.looks(),
		move_rules(moving),
	]


## GDScript's copies after the opening's `flat` result: the tick's perception (returned)
## and the joined squads out of `squads`.
static func mirror(squads: Array, flat: PackedInt64Array) -> Dictionary:
	var at := 1 + 4 * flat[0]  # past the pursuers, then the re-routed slots
	at += 1 + flat[at]
	var perception := {}
	for squad in squads:
		var seen := []
		for index in range(at + 1, at + 1 + flat[at]):
			seen.append(squads[flat[index]])
		at += 1 + flat[at]
		perception[squad.id] = {"seen": seen, "threat": not seen.is_empty()}
	FormationJoins.join(squads, PackedInt32Array(Array(flat.slice(at))))
	return perception


## The move's tuning from `moving` [TRAVEL_SCALE, seconds a tick, the lane's length]: those,
## then discipline_per_leadership, walk_slack_loose, walk_slack_drilled, bodies_ground_drag,
## bodies_ground_block, wounds_reach and walk_lost.
static func move_rules(moving: Array) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(moving)
	(
		rules
		. append_array(
			[
				tuning.discipline_per_leadership,
				tuning.walk_slack_loose,
				tuning.walk_slack_drilled,
				tuning.bodies_ground_drag,
				tuning.bodies_ground_block,
				tuning.wounds_reach,
				tuning.walk_lost,
			]
		)
	)
	return rules
