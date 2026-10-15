class_name ScrumTurn
extends RefCounted
## Turning as a re-form (Decision 92, spec 27 round 6): a squad that must face another way
## where it stands - going back the way it faces, or re-forming after a withdrawal or a
## pursuit - takes its new heading at once, an about-face reversing its ranks, and its units
## walk from where they stand to their new places at its re-form pace
## (FormationDiscipline). It moves on once they are all in place, so a disciplined squad
## turns quickly and a ragged one slowly. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const SquadTurn = preload("res://sim/skirmish/formation/squad_turn.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## Re-faces the squad where it stands before it moves `travel_sign` along its route: an
## about-face first if it must go back the way it faces, then its frame turned to its
## route's heading there. True if it turned; its units then re-form before it moves. (On
## the march a squad doesn't come here for a bend: its frame sweeps round it, FormationSweep.)
static func begin(squad: SkirmishSquad, travel_sign: int, tick: int, events: Array) -> bool:
	var before := squad.heading
	if travel_sign != squad.direction:
		var depth := SquadTurn.reverse_ranks(squad)
		squad.front_distance -= squad.direction * depth * SkirmishSquad.RANK_DEPTH
		squad.direction = -squad.direction
		squad.heading = fposmod(squad.heading + 180.0, 360.0)
	if squad.route != null:
		var cells := squad.front_distance * MapLayoutDef.CELLS_PER_TILE
		var way := squad.route.heading_at(cells) * squad.direction
		squad.heading = UnitMotion.bearing_to(Vector2.ZERO, way, squad.heading)
	if is_equal_approx(squad.heading, before) and travel_sign == squad.direction:
		return false
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if not squad.loose.has(unit.id):
			var stood := at[unit.slot]
			SquadEntries.loosen(squad, unit, {"unit": unit, "goal": null, "next": stood}, stood)
	var faced := {"facing": squad.facing, "heading": squad.heading}
	events.append(FormationEvents.squad_event("turning", tick, squad, faced))
	events.append(FormationEvents.squad_event("turned", tick, squad, faced))
	return true
