class_name FormationOpening
extends RefCounted
## The tick's opening as one crate call (`opening.rs`, spec 39 step 8b5-xxxv): the orders
## queued apply (FormationOrders), each squad sees (FormationPerception), the fronts and then
## the edges lock (FormationFronts, FormationEdges), the groups find their way
## (FormationPathing, on ground only) and the squads move and join (the move's pass and
## FormationJoins), all over one roster, every pass's events in the crate's log in that order.
## The crate names each squad's route by a code it holds (step 8b5-xxxvi), and GDScript's
## copies follow after the call, as each pass's own did: a pursuer takes its quarry's route,
## a group that re-routed its route afresh, what each squad sees becomes the tick's
## perception and the joined squads leave the list.

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
	var tuning := BattleTuning.current()
	var field: Object = squads[0].store.field()
	var flat: PackedInt64Array = field.opening_step(
		CrateRoster.of(squads),
		UnitMover.ground(terrain),
		PackedInt64Array(ticks),
		FormationOrders.rules(),
		BandTuning.rules(),
		PackedInt64Array([tuning.morale_side_impact, tuning.morale_rear_impact]),
		FormationPathing.looks(),
		move_rules(moving)
	)
	events.append_array(CrateEvents.taken(field))
	var at := _pursued(squads, flat, field)
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


## The pursuers' and the re-routed groups' routes taken by GDScript's copies, in the order
## the passes took them: the re-routed are read first, as a pursuer's route written back
## would overwrite one. Returns where in `flat` what each squad sees begins.
static func _pursued(squads: Array, flat: PackedInt64Array, field: Object) -> int:
	var pursuers := flat[0]
	var at := 1 + 4 * pursuers
	var rerouted := {}  # squad -> its route's points as the crate holds them
	for index in range(at + 1, at + 1 + flat[at]):
		for squad in squads:
			if squad.slot == flat[index]:
				rerouted[squad] = field.squad_route(squad.slot)
	for p in range(1, at, 4):
		if flat[p + 3] != 0:
			squads[flat[p + 1]].route = squads[flat[p]].route  # its quarry's (Decision 113)
	for squad in rerouted:
		squad.route = FormationPathing.rerouted(squad, rerouted[squad])
	return at + 1 + flat[at]
