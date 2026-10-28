class_name ScrumTurn
extends RefCounted
## Turning as a re-form (Decision 92, spec 27 round 6): a squad that must face another way
## where it stands - going back the way it faces, or re-forming after a withdrawal or a
## pursuit - takes its new heading at once, an about-face reversing its ranks, and its units
## walk from where they stand to their new places at its re-form pace
## (FormationDiscipline). It moves on once they are all in place, so a disciplined squad
## turns quickly and a ragged one slowly. Pure. The turn, the ranks reversed and the units
## loosened are the crate's (`turn.rs`, spec 39 step 8a2-i); the moves under way cancelled
## and the events stay here.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")

## The crate's answers (turn::KEPT, TURNED and ABOUT_FACED).
const KEPT := 0
const ABOUT_FACED := 2


## Re-faces the squad where it stands before it moves `travel_sign` along its route: an
## about-face first if it must go back the way it faces, then its frame turned to its
## route's heading there. True if it turned; its units then re-form before it moves. (On
## the march a squad doesn't come here for a bend: its frame sweeps round it, FormationSweep.)
static func begin(squad: SkirmishSquad, travel_sign: int, tick: int, events: Array) -> bool:
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	var roster := CrateRoster.of([squad])
	var turned: int = ColumnsLent.all(columns, func(): return field.squad_turn(roster, travel_sign))
	if turned == KEPT:
		return false
	if turned == ABOUT_FACED:
		FormationShuffle.clear(squad)  # moves under way are cancelled: it re-forms
	var faced := {"facing": squad.facing, "heading": squad.heading}
	events.append(FormationEvents.squad_event("turning", tick, squad, faced))
	events.append(FormationEvents.squad_event("turned", tick, squad, faced))
	return true
