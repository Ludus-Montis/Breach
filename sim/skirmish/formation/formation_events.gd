class_name FormationEvents
extends RefCounted
## The plain-dictionary events FormationSimulation.step() returns (for logs and replay
## comparison), per specs/22-formation-feel-test.md. Pure builders.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


static func squad_event(
	kind: String, tick: int, subject: SkirmishSquad, extra: Dictionary = {}
) -> Dictionary:
	var event := {"type": kind, "tick": tick, "squad": subject.id, "faction": subject.faction_id}
	event.merge(extra)
	return event


static func unit_event(
	kind: String, tick: int, owner: SkirmishSquad, unit: SkirmishUnit, extra: Dictionary = {}
) -> Dictionary:
	var event := squad_event(kind, tick, owner, {"unit": unit.id})
	event.merge(extra)
	return event


## blow = [attacker, target, damage, is_flank, how it landed (BlowRoll)], after the damage
## has been applied, leaving the target at `target_hp`.
static func hit(tick: int, blow: Array, target_hp: int) -> Dictionary:
	return {
		"type": "hit",
		"tick": tick,
		"unit": blow[0].id,
		"faction": blow[0].faction_id,
		"target": blow[1].id,
		"dmg": blow[2],
		"flank": blow[3],
		"blow": blow[4],
		"target_hp": target_hp,
	}
