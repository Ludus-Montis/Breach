class_name ScrumSlots
extends RefCounted
## The bodies in the scrum, as the passes still in GDScript read them (spec 39 step 4a):
## every standing unit's body. The slots
## round each foe's body that a seeker makes for (Decisions 88 and 106, spec 30 round 1)
## are the crate's (`scrum.rs`, ScrumSeekField). Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")


## [[where, radius, unit], ...]: every living unit's body in standing squads.
static func bodies(squads: Array) -> Array:
	var out := []
	var views := {}  # store -> where its units stand, read once (Decision 146)
	for squad in squads:
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			continue
		var standing := UnitView.read_once(views, squad, ScrumReach.STANDING)
		for unit in squad.living():
			out.append([ScrumReach.at_in(standing, squad, unit), ScrumReach.radius(unit), unit])
	return out
