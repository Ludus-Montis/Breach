class_name FormationOrders
extends RefCounted
## Orders (Decisions 95, 99 and 109): the wave orders given since the last tick apply
## together, each to its squad if it stands, in the order they were given; who is fighting
## is read before any lands, so orders given together act together. A squad ordered to
## retreat out of a fight disengages at once and withdraws (FormationWithdraw), a ragged one
## paying a scaled rout: a morale shock in proportion to how far short of drilled it is. Its
## enemies may pursue it as a whole (FormationPursuit) and their units chase it on their own
## (ScrumPursuit). Applying them is the crate's (`orders.rs`, spec 39 step 7c): this hands it
## the squads' roster (CrateRoster), and for a retreat each squad's route, whether it may
## pursue and its units' chances to break ranks and leashes (FormationDiscipline), and lands
## what it returns in its order - the shocks, a pursuer's routes and the events.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWithdraw = preload("res://sim/skirmish/formation/formation_withdraw.gd")


## Applies `pending` (squad id -> SkirmishUnit.Order, in the order given) to `squads`.
static func apply(
	squads: Array, pending: Dictionary, tick: int, fight_seed: int, events: Array
) -> void:
	if pending.is_empty() or squads.is_empty():
		return
	var retreats: bool = pending.values().has(SkirmishUnit.Order.RETREAT)
	var given := _given(squads) if retreats else _none()
	var tuning := BattleTuning.current()
	var applied: PackedInt64Array = squads[0].store.field().apply_orders(
		CrateRoster.of(squads),
		PackedInt64Array(pending.keys()),
		PackedInt32Array(pending.values()),
		given[0],
		given[1],
		given[2],
		PackedFloat64Array([tuning.pursuit_tempted_within]),
		tick,
		fight_seed
	)
	var at := 0
	while at < applied.size():
		var squad: SkirmishSquad = squads[applied[at]]
		var order_name: String = SkirmishUnit.Order.keys()[squad.order]
		var named := {"order": order_name}
		events.append(FormationEvents.squad_event("order_applied", tick, squad, named))
		var pursuers: int = applied[at + 1]
		at += 2
		if pursuers >= 0:
			_retreated(squad, applied.slice(at, at + 3 * pursuers), squads, tick, events)
			at += 3 * pursuers


## What a retreat reads of each squad, in the lists' order: [its route's code (the crate
## holds the route), whether it may pursue, its living units' chances and leashes].
static func _given(squads: Array) -> Array:
	var tuning := BattleTuning.current()
	var codes := {}  # route -> its code
	var routes := PackedInt64Array()
	var pursues := PackedByteArray()
	var temptations := PackedFloat64Array()
	for squad: SkirmishSquad in squads:
		routes.append(-1 if squad.route == null else codes.get_or_add(squad.route, codes.size()))
		pursues.append(int(squad.pursues))
		for unit in squad.living():
			var steadied := FormationDiscipline.unit_discipline(squad, unit)  # its leader's too
			var chance := float(tuning.discipline_meets_threats - steadied) / 100.0
			temptations.append_array([chance, FormationDiscipline.unit_leash(squad, unit)])
	return [routes, pursues, temptations]


## What a retreat reads when no order retreats: nothing.
static func _none() -> Array:
	return [PackedInt64Array(), PackedByteArray(), PackedFloat64Array()]


## Lands a retreat out of a fight: its events, a ragged one's shock, and each pursuer's
## (`pursuers`: three numbers each, as apply_orders returns them) routes and event.
static func _retreated(
	squad: SkirmishSquad, pursuers: PackedInt64Array, squads: Array, tick: int, events: Array
) -> void:
	events.append(FormationEvents.squad_event("disengaged", tick, squad))
	var short := FormationWithdraw.disorder(squad)
	if short > 0.0:
		FormationMorale.shock(
			squad, roundi(BattleTuning.current().pursuit_ragged_shock * short), tick, events
		)
	events.append(FormationEvents.squad_event("withdrawing", tick, squad))
	for at in range(0, pursuers.size(), 3):
		var enemy: SkirmishSquad = squads[pursuers[at]]
		if pursuers[at + 1] != 0:  # its post's route, kept till it is back there
			enemy.store.orders.hold_route(enemy.slot, enemy.route)
		if pursuers[at + 2] != 0:
			enemy.route = squad.route  # the route its quarry flees by (Decision 113)
		events.append(FormationEvents.squad_event("pursuing", tick, enemy, {"of": squad.id}))
