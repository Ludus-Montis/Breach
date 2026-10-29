class_name ScrumEngage
extends RefCounted
## Fights start where units meet (Decision 88, spec 27 round 6): with contact-seeking, two hostile
## squads whose units come within reach of each other (reach_engage, BattleTuning) lock into a
## fight, whatever their faces - a squad passing beside a line, or a line it brushes, fights rather
## than walking by. A squad already fighting is joined by the one that reached it, and keeps its own
## foe. Any standing squad may be engaged, one retreating or routing too; only one not getting away
## seeks combat (Decision 111). Every free squad picks the nearest hostile it has come within reach
## of, all from one snapshot before any lock lands, a tie going to the squads' seeded draws - never
## to the order they are listed in (Decision 97). The rules are the crate's (`engage.rs`, spec 39
## step 4b), over the squads' roster, each squad's living units read where they stand, and its
## events in the crate's log (`scrum::pass::engage`, spec 39 step 8b5-xi).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## Locks free squads onto hostile squads their units have come within reach of, on the
## battle's `field` (null: no squads) over its squads' `roster` (CrateRoster). Returns
## "engaged" events.
static func step(field: Object, roster: Object, tick: int, fight_seed: int = 0) -> Array:
	if field == null:
		return []
	field.scrum_engage(roster, tick, fight_seed, BattleTuning.current().reach_engage)
	return CrateEvents.taken(field)
