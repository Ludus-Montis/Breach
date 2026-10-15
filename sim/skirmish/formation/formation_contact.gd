class_name FormationContact
extends RefCounted
## Contact between squads on one lane (specs/22-formation-feel-test.md, Decisions 40 and
## 44). Covers:
## - who can fight, and which hostile front is within melee reach
## - where an advancing squad must stop: at reach of a hostile front, or behind the back
##   rank of a friendly squad ahead (squads never pass through their own side)
## - how a wave that reaches a friendly squad in combat joins it from the back, as rear
##   ranks that step up as the front falls; a wave that merges (Decision 51) joins a
##   friendly squad on the march the same way
## - when a squad without front units holds to skirmish (Decision 47)
## Pure over the squads it is given; FormationSimulation calls it each tick.

const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")

## Tiles between engaged fronts: a little over one cell (Decision 68).
const MELEE_REACH := 0.0175
const EPSILON := 0.000001

## The squads' geometry for one tick's limits (limit): every limit of a tick is judged before
## any move lands, so they share one memo, kept while they are given the same (non-empty)
## `marching`: a new tick's is a new one. SquadMemo's frame guard catches a squad moved
## between two calls given the same.
static var _limits := SquadMemo.new()
static var _limits_for: Variant = null


## True if the squad may be engaged: any squad still standing, whatever it is doing - one
## retreating or routing too. What a squad attacks is the attacker's choice (Decision 111).
## `memo`: the phase's (SquadMemo), if any.
static func engageable(target: SkirmishSquad, memo: SquadMemo = null) -> bool:
	if target.state == SkirmishSquad.State.DESTROYED:
		return false
	return not (memo.living(target) if memo != null else target.living()).is_empty()


## True if the squad's units stand in its frame, so its fronts and edges are where its
## frame is: not a routing squad's, whose units flee on their own - they are met unit by
## unit where they are (ScrumEngage).
static func framed(target: SkirmishSquad) -> bool:
	return target.state != SkirmishSquad.State.ROUTING


## True if the squad seeks combat: not one retreating or routing (it is getting away), nor
## one that has arrived or is waiting (Decision 111).
static func can_engage(candidate: SkirmishSquad) -> bool:
	return (
		candidate.state != SkirmishSquad.State.DESTROYED
		and candidate.state != SkirmishSquad.State.ARRIVED
		and candidate.state != SkirmishSquad.State.ROUTING
		and candidate.order != SkirmishUnit.Order.RETREAT
		and candidate.wait_ticks == 0
	)


## The hostile front within melee reach of `from`'s, facing it: the nearest, a tie going
## to the squads' seeded draws (Decision 97), never to the order they are listed in.
## `memo`: the phase's (SquadMemo).
static func nearest_hostile(
	from: SkirmishSquad, squads: Array, fight_seed: int = 0, memo: SquadMemo = null
) -> SkirmishSquad:
	if memo == null:
		memo = SquadMemo.new()
	var best: SkirmishSquad = null
	var best_key := []
	for other in squads:
		if other.faction_id == from.faction_id or not engageable(other, memo) or not framed(other):
			continue
		if not SquadGeometry.facing_off(from, other) or not memo.overlaps(from, other):
			continue
		var gap := absf(SquadGeometry.gap(from, other))
		var key := [snappedf(gap, EPSILON), ScrumContest.squad_draw(other, fight_seed)]
		if gap <= MELEE_REACH + EPSILON and (best == null or key < best_key):
			best = other
			best_key = key
	return best


## The furthest an advancing squad may get this tick, wanting to reach `next`, judged
## from where everyone stood as the tick began: all squads move together (Decision 97).
## `marching` (squad id -> true) holds the squads advancing this tick; two hostiles
## marching at each other each close at most half the gap that stood between them.
static func limit(
	mover: SkirmishSquad, squads: Array, next: float, marching: Dictionary = {}
) -> float:
	if marching.is_empty() or not is_same(marching, _limits_for):
		_limits = SquadMemo.new()
		_limits_for = marching
	for other in squads:
		if other == mover or not can_engage(other):
			continue
		var room := _room(mover, other, _limits)
		if room == INF:
			continue
		if marching.has(other.id) and _closing_on(mover, other):
			room /= 2.0  # it closes too: each takes its half
		var reachable := mover.front_distance + mover.direction * maxf(room, 0.0)
		next = minf(next, reachable) if mover.direction > 0 else maxf(next, reachable)
	return next


## True if an advancing squad with no front-preferring units has an enemy within its
## ranged reach: it holds there, skirmishing, rather than marching into melee (Decision 47).
## `memo`: the phase's (SquadMemo).
static func skirmishing(mover: SkirmishSquad, squads: Array, memo: SquadMemo = null) -> bool:
	var living := mover.living()
	if (
		mover.order != SkirmishUnit.Order.ADVANCE
		or living.any(func(u): return u.preferred_position == 0)
	):
		return false
	var reach: int = living.reduce(func(most, u): return maxi(most, u.attack_range), 0)
	if reach == 0:
		return false
	if memo == null:
		memo = SquadMemo.new()
	for other in squads:
		if (
			other.faction_id != mover.faction_id
			and can_engage(other)
			and not memo.living(other).is_empty()
			and memo.overlaps(mover, other)
		):
			var gap := absf(SquadGeometry.gap(mover, other))
			if gap <= reach * SkirmishSquad.RANK_DEPTH + EPSILON:
				return true
	return false


## The friendly squad whose back rank `mover` has reached and may join, or null: one in
## combat, or - when `mover` merges - one on the march that isn't retreating. `memo`: the
## phase's (SquadMemo).
static func joinable(mover: SkirmishSquad, squads: Array, memo: SquadMemo = null) -> SkirmishSquad:
	if memo == null:
		memo = SquadMemo.new()
	for other in squads:
		if (
			other == mover
			or other.faction_id != mover.faction_id
			or not _accepts(mover, other)
			or not _ahead(mover, other, memo)
		):
			continue
		if SquadGeometry.gap(mover, other) - _depth(other) <= EPSILON:
			return other
	return null


## Moves the joining squad's living units in behind the leader's back rank, both lines
## centred on the wider of the two, and steps them up. They are the leader's joined units,
## free to spread in a fight (Decision 51). Returns the units that moved up.
static func reinforce(leader: SkirmishSquad, joining: SkirmishSquad) -> Array[SkirmishUnit]:
	var width := maxi(leader.width, joining.width)
	var back := _back_rows(leader)
	var shift := (width - leader.width) / 2
	# store: scattered (a reinforcement: once, as a squad joins)
	for unit in leader.units:
		UnitPlace.put(leader, unit, UnitPlace.of(leader, unit) + Vector2i(0, shift))
	# store: scattered (a reinforcement: once, as a squad joins)
	for unit in joining.living():
		var moved := Vector2i(back, (width - joining.width) / 2)
		UnitPlace.put(joining, unit, UnitPlace.of(joining, unit) + moved)
		unit.attack_cooldown = 1
		SquadEntries.unloose(joining, unit.id)
		SquadEntries.unflee(joining, unit.id)
		FormationCommand.enlist(leader, unit)
		leader.joined.append(unit)
	leader.centre_shift += (width - leader.width) / 2.0 - shift  # the leader stays put
	leader.width = width
	joining.units.clear()
	return leader.compact()


## How far the mover may come toward `other` (tiles): to melee reach of a hostile's front
## or of its side or rear face (Decision 78), or to just behind a friend; INF if `other`
## isn't ahead across the mover's line.
static func _room(mover: SkirmishSquad, other: SkirmishSquad, memo: SquadMemo) -> float:
	if other.faction_id != mover.faction_id and not SquadGeometry.facing_off(mover, other):
		var face := memo.face_gap(mover, other)
		if face > EPSILON and memo.overlap_across(mover, other):
			return face - MELEE_REACH
		return INF
	if not _ahead(mover, other, memo):
		return INF
	if other.faction_id != mover.faction_id:
		return SquadGeometry.gap(mover, other) - MELEE_REACH
	return SquadGeometry.gap(mover, other) - _depth(other)


## True if `other` is a hostile face to face with the mover.
static func _closing_on(mover: SkirmishSquad, other: SkirmishSquad) -> bool:
	return other.faction_id != mover.faction_id and SquadGeometry.facing_off(mover, other)


static func _accepts(mover: SkirmishSquad, leader: SkirmishSquad) -> bool:
	if leader.state == SkirmishSquad.State.FIGHTING:
		return true
	return (
		mover.merges
		and leader.state in [SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING]
		and leader.order != SkirmishUnit.Order.RETREAT
	)


static func _ahead(mover: SkirmishSquad, other: SkirmishSquad, memo: SquadMemo) -> bool:
	return SquadGeometry.gap(mover, other) > EPSILON and memo.overlaps(mover, other)


## How deep a squad is in tiles: a squad behind it stops that far behind its front.
static func _depth(friend: SkirmishSquad) -> float:
	return _back_rows(friend) * SkirmishSquad.RANK_DEPTH


static func _back_rows(squad: SkirmishSquad) -> int:
	var rows := 0
	var places := UnitPlace.table(squad)  # fetched once (spec 35)
	for unit in squad.living():
		rows = maxi(rows, places[0][unit.slot] + unit.footprint_depth)
	return rows
