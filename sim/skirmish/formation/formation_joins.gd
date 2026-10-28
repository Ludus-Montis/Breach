class_name FormationJoins
extends RefCounted
## Waves joining a friend from the back (Decisions 44, 51 and 97): once every squad has
## moved, an advancing wave that has reached a friendly squad's back rank joins it - one in
## combat, or, if it merges, one on the march (FormationContact.joinable). Several reaching
## one friend in a tick join nearest first, so the nearest takes the rank behind it; a tie
## goes to the waves' seeded draws, never to the order they are listed in. Pure over the
## squads it is given. Who joins whom, and in what order, is the crate's (`contact.rs`, spec
## 39 step 8a2-i); the moves too (FormationContact.reinforce); the events and the lists here.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")


## Joins each of `movers` that has reached a friend's back rank into it, and takes it out
## of `squads`. Adds "merged" or "reinforced", and "stepped_up", events.
static func join(squads: Array, movers: Array, tick: int, fight_seed: int, events: Array) -> void:
	if movers.is_empty():
		return
	var indices := PackedInt64Array(movers.map(func(mover): return squads.find(mover)))
	var roster := CrateRoster.of(squads)
	var joins: PackedInt64Array = movers[0].store.field().contact_joins(roster, indices, fight_seed)
	var pairs := []  # [leader, joining], before any join takes a squad out of `squads`
	for at in range(0, joins.size(), 2):
		pairs.append([squads[joins[at]], squads[joins[at + 1]]])
	for pair in pairs:
		_join(squads, pair[0], pair[1], tick, events)


static func _join(
	squads: Array, leader: SkirmishSquad, joining: SkirmishSquad, tick: int, events: Array
) -> void:
	var kind := "reinforced" if leader.state == SkirmishSquad.State.FIGHTING else "merged"
	events.append(FormationEvents.squad_event(kind, tick, joining, {"into": leader.id}))
	# store: scattered (few: the step-ups after a join)
	for moved in FormationContact.reinforce(leader, joining):
		var rank := UnitPlace.rank(leader, moved)
		events.append(FormationEvents.unit_event("stepped_up", tick, leader, moved, {"rank": rank}))
	squads.erase(joining)
	leader.reforming = true
