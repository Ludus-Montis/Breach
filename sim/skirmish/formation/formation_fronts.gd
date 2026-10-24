class_name FormationFronts
extends RefCounted
## Front-to-front locks (Decisions 40 and 97): every free squad picks the nearest hostile
## front within melee reach (a tie going to the squads' draws), all from one snapshot, then
## the locks land together - a squad between two foes isn't taken by whichever was listed
## first. A free foe that picked no one locks onto the nearest of the squads that picked it
## - unless it is getting away, retreating or routing: it may be engaged, but doesn't turn
## to fight (Decision 111). Pure over the squads it is given. The rules are the crate's
## (`contact.rs`, spec 39 step 4c).

const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")


## Locks free squads onto the hostile fronts they reach. Adds "engaged" events. `roster`:
## the tick's (CrateRoster), made here if none is given.
static func engage(
	squads: Array, tick: int, fight_seed: int, events: Array, roster: Object = null
) -> void:
	if squads.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads)
	var picks: PackedInt32Array = squads[0].store.field().contact_fronts(roster, fight_seed)
	for at in range(0, picks.size(), 2):
		var foe: SkirmishSquad = squads[picks[at + 1]]
		events.append(
			FormationEvents.squad_event("engaged", tick, squads[picks[at]], {"with": foe.id})
		)
