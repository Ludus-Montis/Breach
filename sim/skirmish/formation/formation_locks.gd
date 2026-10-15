class_name FormationLocks
extends RefCounted
## Which squads are locked in a fight (Decisions 40 and 47): a lock makes a squad fight its
## foe from this tick; a release frees it and everyone fighting it. A turning squad keeps
## turning through either and takes up the fight when the turn ends (Decision 74).
## Pure over the squads it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


static func lock(squad: SkirmishSquad, foe: SkirmishSquad) -> void:
	squad.engaged_with = foe.id
	squad.state = SkirmishSquad.State.FIGHTING
	if not squad.joined.is_empty():
		squad.reforming = true  # units merged on the march may now spread
	var cooldowns: PackedInt32Array = squad.store.columns.column(UnitColumns.ATTACK_COOLDOWN)
	for unit in squad.living():
		cooldowns[unit.slot] = 1  # the first blows land this tick
	squad.store.columns.commit(UnitColumns.ATTACK_COOLDOWN, cooldowns)


## Ends a squad's fight and frees every squad that was fighting it, front or flank.
static func release(released: SkirmishSquad, squads: Array) -> void:
	_free(released)
	for other in squads:
		if other.engaged_with == released.id:
			_free(other)
		for edge in other.flank_contacts.keys():
			if other.flank_contacts[edge]["foe"] == released.id:
				other.flank_contacts.erase(edge)
		if other.flank_contacts.is_empty() and other.engaged_with == 0:
			if other.state == SkirmishSquad.State.FIGHTING:
				other.state = SkirmishSquad.State.MOVING


static func _free(squad: SkirmishSquad) -> void:
	squad.engaged_with = 0
	if squad.state == SkirmishSquad.State.FIGHTING and squad.flank_contacts.is_empty():
		squad.state = SkirmishSquad.State.MOVING
