class_name FormationDeaths
extends RefCounted
## The fallen (specs/22-formation-feel-test.md, Decisions 47, 82 and 121): a unit brought to
## 0 hp is downed - its hp stops at 0, the excess lost, and it lies where it fell - unless
## the blows took it past minus its constitution, which kills it outright. Either way it
## leaves the fight and shakes its squad, the ranks behind step up (units find the fight
## themselves: Decision 88), and a squad with no one left is destroyed, freeing whoever
## fought it. What becomes of the downed is FormationWounds'. Pure over the squads.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")


static func bury(squads: Array, tick: int, events: Array) -> void:
	for fallen_squad: SkirmishSquad in squads:
		var fallen := []
		for unit in fallen_squad.living():
			if unit.hp > 0:
				continue
			fallen.append(unit)
			if unit.hp <= -depth(unit):
				unit.state = SkirmishUnit.State.DEAD
				events.append(FormationEvents.unit_event("died", tick, fallen_squad, unit))
			else:
				unit.hp = 0
				unit.state = SkirmishUnit.State.DOWNED
				unit.wounded += 1 + unit.last_wounding  # lowering its condition (Decision 126)
				unit.wake_left = -1.0
				events.append(FormationEvents.unit_event("downed", tick, fallen_squad, unit))
		if fallen.is_empty():
			continue
		FormationMorale.losses(fallen_squad, fallen, tick, events)
		for moved in fallen_squad.compact():
			var extra := {"rank": UnitPlace.rank(fallen_squad, moved)}
			events.append(
				FormationEvents.unit_event("stepped_up", tick, fallen_squad, moved, extra)
			)
		fallen_squad.reforming = true  # front units may close gaps (Decision 49)
		if fallen_squad.is_destroyed() and fallen_squad.state != SkirmishSquad.State.DESTROYED:
			fallen_squad.state = SkirmishSquad.State.DESTROYED
			FormationLocks.release(fallen_squad, squads)
			events.append(FormationEvents.squad_event("destroyed", tick, fallen_squad))


## How far below 0 hp a unit lives on, at death's door: its constitution (Decision 121).
static func depth(unit: SkirmishUnit) -> int:
	return int(unit.attributes.get("constitution", UnitDef.AVERAGE))
