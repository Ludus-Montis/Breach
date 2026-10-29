class_name FormationJoins
extends RefCounted
## Waves joining a friend from the back (Decisions 44, 51 and 97): once every squad has
## moved, an advancing wave that has reached a friendly squad's back rank joins it - one in
## combat, or, if it merges, one on the march (FormationContact.joinable). Several reaching
## one friend in a tick join nearest first, so the nearest takes the rank behind it; a tie
## goes to the waves' seeded draws, never to the order they are listed in. The joins are the
## crate's (`joins.rs`, spec 39 step 8b5-x), with the move (`move_step`): who joins whom, the
## moves, the members, the units' command and the events. GDScript's own lists follow here.

const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")


## The crate's `joins` - leader then joining, as indices into `squads`, a pair a join, in the
## order joined - taken by GDScript's lists: each joining squad's units are its leader's
## (FormationContact.took_in), and it leaves `squads`.
static func join(squads: Array, joins: PackedInt32Array) -> void:
	var pairs := []  # [leader, joining], before any join takes a squad out of `squads`
	for at in range(0, joins.size(), 2):
		pairs.append([squads[joins[at]], squads[joins[at + 1]]])
	for pair in pairs:
		FormationContact.took_in(pair[0], pair[1])
		squads.erase(pair[1])
