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
## step 4b): this hands it where each squad's living units stand. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")


## Locks free squads onto hostile squads their units have come within reach of. Returns
## "engaged" events.
static func step(squads: Array, tick: int, fight_seed: int = 0) -> Array:
	if squads.is_empty():
		return []
	var units := PackedInt32Array()
	var ats := PackedVector2Array()
	var feet := PackedInt32Array()
	var counts := PackedInt32Array()
	var joined := PackedByteArray()
	var views := {}  # store -> where its units stand (ScrumReach.standing): nobody moves here
	for squad in squads:
		var living: Array = squad.living()
		var standing := UnitView.read_once(views, squad, ScrumReach.STANDING)
		for unit in living:
			units.append(unit.slot)
			ats.append(ScrumReach.at_in(standing, squad, unit))
			feet.append_array([unit.footprint_width, unit.footprint_depth])
		counts.append(living.size())
		joined.append(int(not squad.joined.is_empty()))
	var picks: PackedInt64Array = squads[0].store.field().engage(
		FormationLocks.slots(squads),
		FormationLocks.factions(squads),
		joined,
		counts,
		units,
		ats,
		feet,
		BattleTuning.current().reach_engage,
		fight_seed
	)
	var events := []
	for index in range(0, picks.size(), 2):
		var squad: SkirmishSquad = squads[picks[index]]
		events.append(
			FormationEvents.squad_event("engaged", tick, squad, {"with": picks[index + 1]})
		)
	return events
