class_name FormationEdges
extends RefCounted
## Fights on a squad's sides and rear (Decision 78, spec 27 round 2). An attacker whose
## front reaches a hostile squad's side or rear face locks on (a flank lock): the victim
## records the contact on that edge, stops, and takes the shock of it. The units then
## fight it out in the scrum (Decision 88), which also ends the lock.
## Nothing hangs on list order (Decision 97): every attacker picks the nearest face it
## reaches from one snapshot; two reaching one edge at once, the nearer holds it.
## Pure over the squads it is given; FormationSimulation calls it each tick.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")

const EPSILON := 0.000001


## Free attackers whose fronts reach a hostile's side or rear lock on, all decided before
## any lands: nearest first, so the nearest attacker holds an edge two reach at once.
static func engage(squads: Array, tick: int, events: Array, fight_seed: int = 0) -> void:
	var picks := []  # [key, attacker, victim]
	var memo := SquadMemo.new()  # every pick is made before any lock lands
	for attacker in squads:
		if _free(attacker):
			var pick := _target(attacker, squads, fight_seed, memo)
			if not pick.is_empty():
				picks.append(pick)
	picks.sort_custom(func(a, b): return a[0] < b[0])
	var held := {}  # [victim, edge] -> true: edges taken this tick
	for pick in picks:
		_lock_on(pick[1], pick[2], tick, events, held)


## [key, attacker, victim] for the nearest side or rear face the attacker reaches (ties by
## the squads' draws), or [].
static func _target(
	attacker: SkirmishSquad, squads: Array, fight_seed: int, memo: SquadMemo
) -> Array:
	var best := []
	var mine := ScrumContest.squad_draw(attacker, fight_seed)
	for victim in squads:
		if victim.faction_id == attacker.faction_id or not _reaches(attacker, victim, memo):
			continue
		var gap := snappedf(memo.face_gap(attacker, victim), EPSILON)
		var key := [gap, mine, ScrumContest.squad_draw(victim, fight_seed)]
		if best.is_empty() or key < best[0]:
			best = [key, attacker, victim]
	return best


static func _lock_on(
	attacker: SkirmishSquad, victim: SkirmishSquad, tick: int, events: Array, held: Dictionary
) -> void:
	var edge := SquadEdges.edge_hit(victim, attacker.heading)
	FormationLocks.lock(attacker, victim)
	if not held.has([victim, edge]):  # a nearer attacker already holds it this tick
		held[[victim, edge]] = true
		victim.flank(edge, attacker.id, tick)
	victim.state = SkirmishSquad.State.FIGHTING
	var cooldowns: PackedInt32Array = victim.store.columns.column(UnitColumns.ATTACK_COOLDOWN)
	for unit in SquadEdges.edge_units(victim, edge):
		cooldowns[unit.slot] = 1
	victim.store.columns.commit(UnitColumns.ATTACK_COOLDOWN, cooldowns)
	var extra := {"by": attacker.id, "edge": edge}
	events.append(FormationEvents.squad_event("flanked", tick, victim, extra))
	var impact := (
		BattleTuning.current().morale_rear_impact
		if edge == SquadEdges.REAR
		else BattleTuning.current().morale_side_impact
	)
	FormationMorale.shock(victim, impact, tick, events)


static func _free(attacker: SkirmishSquad) -> bool:
	return FormationContact.can_engage(attacker) and attacker.engaged_with == 0


## True if the attacker's front reaches a hostile's side or rear face, overlapping it.
static func _reaches(attacker: SkirmishSquad, victim: SkirmishSquad, memo: SquadMemo) -> bool:
	if not FormationContact.engageable(victim, memo) or not FormationContact.framed(victim):
		return false
	if SquadGeometry.facing_off(attacker, victim):
		return false  # front to front: FormationContact's frontal lock
	var gap := memo.face_gap(attacker, victim)
	return (
		gap > -EPSILON
		and gap <= FormationContact.MELEE_REACH + EPSILON
		and memo.overlap_across(attacker, victim)
	)
