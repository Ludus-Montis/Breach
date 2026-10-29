class_name FormationPursuit
extends RefCounted
## A formation pursuing as a whole (Decision 95, spec 27 round 8): when its enemy retreats
## it stays locked on it and its frame advances after it as a body, along the route its
## quarry flees by (Decision 113: a route is a way to travel, not the formation's own) - no
## faster than its rearmost unit keeps up (Decision 103) - its units seeking contact as
## they go at the march pace. When the enemy is out of its sight, gone, or no longer
## retreating, or it has gone as far from its post as its discipline leashes it
## (FormationDiscipline.pursuit_leash, Decision 107), or most of it has tired
## (FormationStamina), it gives up: it marches back along the
## route it is on to the post it held, takes up its own route there and turns to face the
## way it held it, taking up its order again. Squads keep `pursuit` ({"foe", "post",
## "post_at", "home", "direction", "order", "returning"}), a view of their records
## (OrderRecords). Its start is the crate's orders (FormationOrders), and the rest is the
## crate's too (`pursuit/whole.rs`, spec 39 step 7c; `scrum::pursue`, step 8b5-xvi): each
## pursuer's routes (the one it is on, and the one it held its post on, kept as its pursuit
## began) and its leash read where the crate holds them, the pursuit, its own route taken up
## again and the turn back at its post, and "pursuit_ended", "turning" and "turned" in the
## crate's log. This hands it the squads' roster (CrateRoster) and mirrors the route a squad
## back at its post holds. Pure over the squads.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")


## One tick of pursuit: pursuers advance after their enemy or give up and go back.
## `roster`: the tick's (CrateRoster), made here if none is given.
static func step(
	squads: Array,
	tick: int,
	cells_per_second: float,
	seconds: float,
	events: Array,
	roster: Object = null
) -> void:
	if squads.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads)
	var field: Object = squads[0].store.field()
	var back: PackedInt32Array = field.scrum_pursue(roster, rules(cells_per_second, seconds), tick)
	events.append_array(CrateEvents.taken(field))
	for index in back:
		mirror(squads[index], field)


## GDScript's copy of the route the squad is back on, as the crate set it: the one it or its
## command holds with the same points, else one made from them.
static func mirror(squad: SkirmishSquad, field: Object) -> void:
	var points: PackedVector2Array = field.squad_route(squad.slot)
	if points.is_empty():
		squad.route = null
		return
	if squad.route != null and squad.route.points() == points:
		return
	var command: FormationRoute = squad.command.route
	if command != null and command.points() == points:
		squad.route = command
		return
	var half_width := 0.0 if command == null else command.corridor_half_width
	squad.route = FormationRoute.new(points, half_width)


## [cells its frame has gone from its post, as the crow flies, cells its leash allows (INF:
## none)] for a pursuing squad.
static func reach(squad: SkirmishSquad) -> Array:
	var gone: float = squad.position.distance_to(squad.pursuit["post_at"])
	return [gone, FormationDiscipline.pursuit_leash(squad)]


## The tuning the pursuit reads (BodyField's `scrum_pursue`): `cells_per_second` at speed 1
## and the tick's `seconds`, then the pursuit's and the leash's.
static func rules(cells_per_second: float, seconds: float) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var out := PackedFloat64Array(
		[
			cells_per_second,
			seconds,
			tuning.stamina_give_up,
			tuning.reach_pursuit_lag,
			tuning.discipline_per_leadership,
			tuning.discipline_expected_max,
			tuning.discipline_leash_shares.size(),
		]
	)
	out.append_array(PackedFloat64Array(tuning.discipline_leash_shares))
	out.append_array(PackedFloat64Array(tuning.discipline_leash_steps))
	return out
