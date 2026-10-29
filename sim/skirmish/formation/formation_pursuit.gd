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
## way it held it, taking up its order again. Squads keep `pursuit` ({"foe", "route",
## "post", "post_at", "home", "direction", "order", "returning"}), a view of their records
## (OrderRecords). Its start is the crate's orders (FormationOrders), and the rest is the
## crate's too (`pursuit/whole.rs`, spec 39 step 7c): this hands it the squads' roster
## (CrateRoster), each pursuer's routes and leash, and lands what it returns in its order -
## the routes, the turn back at its post (ScrumTurn) and the events. Pure over the squads.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const ScrumTurn = preload("res://sim/skirmish/formation/scrum_turn.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
## The kinds of what the crate's pursuit did (pursue's records).
const ENDED := 0
const ARRIVED := 1


## One tick of pursuit: pursuers advance after their enemy or give up and go back.
## `roster`: the tick's (CrateRoster), made here if a squad pursues and none is given.
static func step(
	squads: Array,
	tick: int,
	cells_per_second: float,
	seconds: float,
	events: Array,
	roster: Object = null
) -> void:
	var listed := PackedInt32Array()
	var leashes := PackedFloat64Array()
	var counts := PackedInt32Array()
	var points := PackedVector2Array()
	var posts := {}  # its index -> the route it held its post on
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		var held: Dictionary = squad.pursuit
		if held.is_empty() or squad.is_destroyed():
			continue
		listed.append(index)
		posts[index] = held["route"]
		leashes.append(FormationDiscipline.pursuit_leash(squad))
		var post: FormationRoute = held["route"]  # its post's; the crate holds the one it is on
		counts.append(-1 if post == null else post.points().size())
		points.append_array(PackedVector2Array() if post == null else post.points())
	if listed.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads)
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[cells_per_second, seconds, tuning.stamina_give_up, tuning.reach_pursuit_lag]
	)
	var done: PackedInt64Array = squads[0].store.field().pursue(
		roster, listed, leashes, counts, points, rules
	)
	for at in range(0, done.size(), 3):
		var squad: SkirmishSquad = squads[done[at + 1]]
		if done[at] == ENDED:
			events.append(FormationEvents.squad_event("pursuit_ended", tick, squad))
			continue
		squad.route = posts[done[at + 1]]  # at its post: its own route again
		ScrumTurn.begin(squad, done[at + 2], tick, events)


## [cells its frame has gone from its post, as the crow flies, cells its leash allows (INF:
## none)] for a pursuing squad.
static func reach(squad: SkirmishSquad) -> Array:
	var gone: float = squad.position.distance_to(squad.pursuit["post_at"])
	return [gone, FormationDiscipline.pursuit_leash(squad)]
