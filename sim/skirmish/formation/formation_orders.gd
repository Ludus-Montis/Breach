class_name FormationOrders
extends RefCounted
## Orders (Decisions 95, 99 and 109): the wave orders given since the last tick apply
## together, each to its squad if it stands, in the order they were given; who is fighting
## is read before any lands, so orders given together act together. A squad ordered to
## retreat out of a fight disengages at once and withdraws (FormationWithdraw), a ragged one
## paying a scaled rout: a morale shock in proportion to how far short of drilled it is. Its
## enemies may pursue it as a whole (FormationPursuit) and their units chase it on their own
## (ScrumPursuit). Applying them is the crate's (`orders.rs`, spec 39 step 7c), and so are
## what a retreat reads (whether a squad may pursue, its units' chances to break ranks and
## leashes: FormationDiscipline's rules, on their sheets), the shock and the events, built as
## data (`orders/pass.rs`, step 8b5-v): this hands it the squads' roster (CrateRoster) and,
## for a retreat, each squad's route, takes its events (CrateEvents) and lands the routes its
## pursuers keep or take.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## Applies the orders the crate has queued, `pending` ([squad id, SkirmishUnit.Order] each, in
## the order given), to `squads`; the crate takes them from its queue.
static func apply(
	squads: Array, pending: PackedInt64Array, tick: int, fight_seed: int, events: Array
) -> void:
	if pending.is_empty() or squads.is_empty():
		return
	var retreats := false
	for at in range(1, pending.size(), 2):
		retreats = retreats or pending[at] == SkirmishUnit.Order.RETREAT
	var field: Object = squads[0].store.field()
	var pursuers: PackedInt64Array = field.apply_orders(
		CrateRoster.of(squads),
		_routes(squads) if retreats else PackedInt64Array(),
		_rules(),
		BandTuning.rules(),
		tick,
		fight_seed
	)
	events.append_array(CrateEvents.taken(field))
	for at in range(0, pursuers.size(), 4):
		var quarry: SkirmishSquad = squads[pursuers[at]]
		var enemy: SkirmishSquad = squads[pursuers[at + 1]]
		if pursuers[at + 2] != 0:  # its post's route, kept till it is back there
			enemy.store.orders.hold_route(enemy.slot, enemy.route)
		if pursuers[at + 3] != 0:
			enemy.route = quarry.route  # the route its quarry flees by (Decision 113)


## A code naming each squad's route, in the lists' order (-1: none); the crate holds them.
static func _routes(squads: Array) -> PackedInt64Array:
	var codes := {}  # route -> its code
	var routes := PackedInt64Array()
	for squad: SkirmishSquad in squads:
		routes.append(-1 if squad.route == null else codes.get_or_add(squad.route, codes.size()))
	return routes


## The tuning the pass reads: the pursuit's, then FormationDiscipline's.
static func _rules() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array([tuning.pursuit_tempted_within, tuning.pursuit_ragged_shock])
	(
		rules
		. append_array(
			PackedFloat64Array(
				[
					tuning.discipline_per_leadership,
					tuning.discipline_meets_threats,
					tuning.discipline_expected_max,
					tuning.discipline_leash_shares.size(),
				]
			)
		)
	)
	rules.append_array(PackedFloat64Array(tuning.discipline_leash_shares))
	rules.append_array(PackedFloat64Array(tuning.discipline_leash_steps))
	return rules
