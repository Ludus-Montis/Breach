class_name ScrumTrade
extends RefCounted
## Trading places while regrouping (Decision 110): a unit walking back to its place that has
## come no nearer than its closest for scrum_trade_seconds - its own ranks in the way - trades
## places with the friend whose place is nearest it, if that place is nearer it than its
## own and the two are interchangeable (one kind, one band); the friend walks to the place
## it left. With no such friend, its own place is the nearest: it takes it. Places stay one
## unit each. Stalled units trade in the order of their seeded draws, never the list's
## (Decision 97). Pure over the squad it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

## Cells nearer than its closest yet that count as getting nearer.
const PROGRESS := 0.001


## Notes how near the loose unit's `entry` stands to its place, `gap` cells off.
static func track(entry: Dictionary, gap: float, seconds: float) -> void:
	if gap < entry.get("closest", INF) - PROGRESS:
		entry["closest"] = gap
		entry["stuck"] = 0.0
	else:
		entry["stuck"] = entry.get("stuck", 0.0) + seconds


## Each stalled unit of the squad trades places, if a friend's place is nearer it; one that
## can't is as near its place as the press allows, and takes it there and then.
static func trade(squad: SkirmishSquad, fight_seed: int) -> void:
	var stalled := []
	for unit_id in squad.loose:
		if (
			squad.loose[unit_id].get("stuck", 0.0)
			>= BattleTuning.current().scrum_trade_seconds - 0.000001
		):
			stalled.append(squad.loose[unit_id]["unit"])
	if stalled.is_empty():
		return
	var draws := {}  # unit -> its draw, worked out once before sorting
	for unit in stalled:
		draws[unit] = ScrumContest.draw(unit, fight_seed)
	stalled.sort_custom(func(a, b): return draws[a] < draws[b])
	var places := _places(squad, stalled)
	for unit in stalled:
		if not squad.loose.has(unit.id):
			continue
		var at: Vector2 = squad.loose[unit.id]["at"]
		var friend := _nearest_place(squad, unit, at, [places, fight_seed])
		if friend != null:
			_swap(squad, unit, friend)
			_traded(places, unit, friend)
		elif squad.stance.is_empty():
			squad.loose.erase(unit.id)
			var anchor := ScrumStance.anchor(squad, unit)
			# placement: it takes its place in its frame (Decision 139)
			squad.store.columns.put(unit.slot, UnitColumns.POSITION, anchor)


## Notes in `places` (_places) that the two traded places: anchors stay with the places.
static func _traded(places: Dictionary, unit: SkirmishUnit, friend: SkirmishUnit) -> void:
	if places.is_empty():
		return
	var held: Array = places["held"]
	var mine: int = places["of"][unit]
	var theirs: int = places["of"][friend]
	held[mine] = friend
	held[theirs] = unit
	places["of"][unit] = theirs
	places["of"][friend] = mine


## The squad's places as its living units hold them, found by where they stand: {"points":
## each one's anchor, "grid" over them, "held": who holds each, "of": unit -> its place,
## "order": unit -> where it comes in the living units}. Places trade only between
## interchangeable units, whose anchors at a place are the same. {} if one of `stalled`
## holds none of them: then every living unit is looked through.
static func _places(squad: SkirmishSquad, stalled: Array) -> Dictionary:
	var living := squad.living()
	var points := []
	var of := {}
	var order := {}
	for index in range(living.size()):
		points.append(ScrumStance.anchor(squad, living[index]))
		of[living[index]] = index
		order[living[index]] = index
	if stalled.any(func(unit): return not of.has(unit)):
		return {}
	return {
		"points": points, "grid": BodyGrid.build(points), "held": living, "of": of, "order": order
	}


## The interchangeable friend whose place is nearest `at`, and nearer than the unit's own;
## null if none. Ties by the friends' draws (then the living units' order, as looking
## through them all). `lookup`: [_places, the fight seed].
static func _nearest_place(
	squad: SkirmishSquad, unit: SkirmishUnit, at: Vector2, lookup: Array
) -> SkirmishUnit:
	var places: Dictionary = lookup[0]
	var fight_seed: int = lookup[1]
	var own := ScrumStance.anchor(squad, unit).distance_to(at)
	var best: SkirmishUnit = null
	var best_key := []
	if places.is_empty():
		places = _places(squad, [])  # as the squad's units stand now
	for index in BodyGrid.near(places["grid"], at, own + BodyGrid.MARGIN):
		var friend: SkirmishUnit = places["held"][index]
		var same: bool = (  # interchangeable: the painted layout keeps its meaning
			friend.definition == unit.definition
			and friend.preferred_position == unit.preferred_position
			and friend.footprint_width == unit.footprint_width
			and friend.footprint_depth == unit.footprint_depth
		)
		if friend == unit or not same or squad.chasers.has(friend.id):
			continue
		var there: float = places["points"][index].distance_to(at)
		if there >= own - PROGRESS:
			continue
		var key := [
			snappedf(there, 0.000001),
			ScrumContest.draw(friend, fight_seed),
			places["order"][friend],
		]
		if best_key.is_empty() or key < best_key:
			best = friend
			best_key = key
	return best


## The two swap places; the friend, loose if it wasn't, walks to its new one.
static func _swap(squad: SkirmishSquad, unit: SkirmishUnit, friend: SkirmishUnit) -> void:
	var place := Vector2i(unit.rank, unit.column)
	unit.rank = friend.rank
	unit.column = friend.column
	friend.rank = place.x
	friend.column = place.y
	if not squad.loose.has(friend.id):
		var at: Vector2 = squad.store.columns.value(friend.slot, UnitColumns.POSITION)
		squad.loose[friend.id] = {"unit": friend, "at": at, "goal": null, "next": at}
	for moved in [unit, friend]:
		squad.loose[moved.id].erase("closest")
		squad.loose[moved.id].erase("stuck")
