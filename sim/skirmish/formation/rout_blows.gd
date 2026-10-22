class_name RoutBlows
extends RefCounted
## Blows on routers (Decision 82): any hostile front-rank unit free to fight that comes
## within rout_strike_reach (BattleTuning) of a router strikes it from behind, a flank blow. It
## strikes the nearest router it reaches, of whichever routing formation, a tie going to
## the routers' seeded draws (Decision 97), and strikes once a tick however many it
## reaches. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")


## Blows on routers this tick: [[attacker, target, damage, flank], ...]. Updates each
## striker's target and cooldown.
static func blows(squads: Array, interval: int, fight_seed: int = 0) -> Array:
	var routers := []  # [unit, where it stands, its faction]
	var routes := {}  # store -> its routers' ROUTE, read once a pass (Decision 150)
	for routing in squads:
		if routing.state == SkirmishSquad.State.ROUTING:
			var route := UnitView.read_once(routes, routing, FormationRout.ROUTE)
			for unit in routing.living():
				routers.append(
					[unit, FormationRout.where_in(route, routing, unit), routing.faction_id]
				)
	if routers.is_empty():
		return []
	var out := []
	var near := BodyGrid.build(routers.map(func(router): return router[1]))
	var reach := BattleTuning.current().rout_strike_reach + BodyGrid.MARGIN
	var cooldowns: PackedInt32Array = squads[0].store.columns.column(UnitColumns.ATTACK_COOLDOWN)
	var targets: PackedInt64Array = squads[0].store.columns.column(UnitColumns.TARGET_ID)
	var standing := {}  # store -> where its units stand, read once a pass: nobody moves here
	for hunter in squads:
		if not FormationContact.can_engage(hunter):
			continue
		var at: PackedVector2Array = (
			UnitView.read_once(standing, hunter, [UnitColumns.POSITION]).position
		)
		for fighter in hunter.fighters():
			var here := at[fighter.slot]
			var close := BodyGrid.near(near, here, reach).map(func(i): return routers[i])
			var target := _nearest([fighter, here], hunter.faction_id, close, fight_seed)
			if target == null:
				continue
			targets[fighter.slot] = target.id
			cooldowns[fighter.slot] -= 1
			if cooldowns[fighter.slot] <= 0:
				cooldowns[fighter.slot] = maxi(1, roundi(interval * fighter.melee_seconds))
				out.append([fighter, target, FormationCombat.damage(fighter), true])
	squads[0].store.columns.commit(UnitColumns.ATTACK_COOLDOWN, cooldowns)
	squads[0].store.columns.commit(UnitColumns.TARGET_ID, targets)
	return out


## The nearest hostile router within reach of the fighter, or null.
## The nearest router to `fighter` = [the fighter, where it stands] not of `faction`.
static func _nearest(
	fighter: Array, faction: String, routers: Array, fight_seed: int
) -> SkirmishUnit:
	var best: SkirmishUnit = null
	var best_key := []
	var here: Vector2 = fighter[1]
	for router in routers:
		if router[2] == faction:
			continue
		var gap := here.distance_to(router[1])
		var key := [snappedf(gap, 0.000001), ScrumContest.draw(router[0], fight_seed)]
		if gap <= BattleTuning.current().rout_strike_reach and (best == null or key < best_key):
			best = router[0]
			best_key = key
	return best
