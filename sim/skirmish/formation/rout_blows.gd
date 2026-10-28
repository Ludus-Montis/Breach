class_name RoutBlows
extends RefCounted
## Blows on routers (Decision 82): any hostile front-rank unit free to fight that comes
## within rout_strike_reach (BattleTuning) of a router strikes it from behind, a flank blow. It
## strikes the nearest router it reaches, of whichever routing formation, a tie going to
## the routers' seeded draws (Decision 97), and strikes once a tick however many it
## reaches. Who it strikes, its target and its cooldown are the crate's (`rout_blows.rs`,
## spec 39 step 7a): this hands it the squads' roster (CrateRoster), each routing squad's
## route, and the fighters of the squads free to fight with the cooldown each starts again
## at, and makes the blows. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## Blows on routers this tick: [[attacker, target, damage, flank], ...]. Updates each
## striker's target and cooldown.
static func blows(squads: Array, interval: int, fight_seed: int = 0) -> Array:
	var routers := []  # each routing squad's living, as the crate indexes them
	var routing := PackedInt32Array()  # each routing squad's index, on the route it holds
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if squad.state != SkirmishSquad.State.ROUTING:
			continue
		routers.append(squad.living())
		routing.append(index)
	if routers.all(func(living): return living.is_empty()):
		return []
	var fighters := []
	var hunting := PackedInt64Array()
	for index in squads.size():
		var hunter: SkirmishSquad = squads[index]
		if not FormationContact.can_engage(hunter):
			continue
		for fighter in hunter.fighters():
			fighters.append(fighter)
			var reset := maxi(1, roundi(interval * fighter.melee_seconds))
			hunting.append_array([index, fighter.slot, reset])
	if fighters.is_empty():
		return []
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var roster := CrateRoster.of(squads)
	var reach := BattleTuning.current().rout_strike_reach
	var struck: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.rout_blows(roster, routing, hunting, reach, fight_seed)
	)
	var out := []
	for at in range(0, struck.size(), 3):
		var fighter: SkirmishUnit = fighters[struck[at]]
		var target: SkirmishUnit = routers[struck[at + 1]][struck[at + 2]]
		out.append([fighter, target, FormationCombat.damage(fighter), true])
	return out
