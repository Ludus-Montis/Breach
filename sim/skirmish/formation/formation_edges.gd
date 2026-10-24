class_name FormationEdges
extends RefCounted
## Fights on a squad's sides and rear (Decision 78, spec 27 round 2). An attacker whose
## front reaches a hostile squad's side or rear face locks on (a flank lock): the victim
## records the contact on that edge, stops, and takes the shock of it. The units then
## fight it out in the scrum (Decision 88), which also ends the lock.
## Nothing hangs on list order (Decision 97): every attacker picks the nearest face it
## reaches from one snapshot; two reaching one edge at once, the nearer holds it.
## Pure over the squads it is given; FormationSimulation calls it each tick. The rules are
## the crate's (`contact.rs`, spec 39 step 4c): this adds each lock's event and shock.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")


## Free attackers whose fronts reach a hostile's side or rear lock on, all decided before
## any lands: nearest first, so the nearest attacker holds an edge two reach at once; the
## victim takes the shock of each. `roster`: the tick's (CrateRoster), made here if none is
## given.
static func engage(
	squads: Array, tick: int, events: Array, fight_seed: int = 0, roster: Object = null
) -> void:
	if squads.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads)
	var flanked: PackedInt32Array = squads[0].store.field().contact_edges(roster, tick, fight_seed)
	for at in range(0, flanked.size(), 3):
		var victim: SkirmishSquad = squads[flanked[at]]
		var edge: int = flanked[at + 2]
		var attacker: SkirmishSquad = squads[flanked[at + 1]]
		var extra := {"by": attacker.id, "edge": edge}
		events.append(FormationEvents.squad_event("flanked", tick, victim, extra))
		var impact := (
			BattleTuning.current().morale_rear_impact
			if edge == SquadEdges.REAR
			else BattleTuning.current().morale_side_impact
		)
		FormationMorale.shock(victim, impact, tick, events)
