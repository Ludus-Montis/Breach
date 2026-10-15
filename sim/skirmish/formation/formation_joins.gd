class_name FormationJoins
extends RefCounted
## Waves joining a friend from the back (Decisions 44, 51 and 97): once every squad has
## moved, an advancing wave that has reached a friendly squad's back rank joins it - one in
## combat, or, if it merges, one on the march (FormationContact.joinable). Several reaching
## one friend in a tick join nearest first, so the nearest takes the rank behind it; a tie
## goes to the waves' seeded draws, never to the order they are listed in. Pure over the
## squads it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")


## Joins each of `movers` that has reached a friend's back rank into it, and takes it out
## of `squads`. Adds "merged" or "reinforced", and "stepped_up", events.
static func join(squads: Array, movers: Array, tick: int, fight_seed: int, events: Array) -> void:
	var joins := []  # [key, leader, joining]
	for mover in movers:
		var leader := FormationContact.joinable(mover, squads)
		if leader != null:
			var gap := snappedf(SquadGeometry.gap(mover, leader), 0.000001)
			joins.append([[gap, ScrumContest.squad_draw(mover, fight_seed)], leader, mover])
	joins.sort_custom(func(a, b): return a[0] < b[0])
	for entry in joins:
		_join(squads, entry[1], entry[2], tick, events)


static func _join(
	squads: Array, leader: SkirmishSquad, joining: SkirmishSquad, tick: int, events: Array
) -> void:
	var kind := "reinforced" if leader.state == SkirmishSquad.State.FIGHTING else "merged"
	events.append(FormationEvents.squad_event(kind, tick, joining, {"into": leader.id}))
	for moved in FormationContact.reinforce(leader, joining):
		var rank := UnitPlace.rank(leader, moved)
		events.append(FormationEvents.unit_event("stepped_up", tick, leader, moved, {"rank": rank}))
	squads.erase(joining)
	leader.reforming = true
