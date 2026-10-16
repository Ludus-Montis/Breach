class_name ScrumBlows
extends RefCounted
## Melee blows in the scrum (Decisions 88 and 95, spec 27 rounds 5 and 8): every unit of a
## fighting squad - and of any squad, at a retreating enemy - strikes one enemy it touches
## (ScrumReach), on a face or a corner: one in its front first, then the nearest, then by
## the battle's seeded draw (ScrumContest; never by id, Decision 97). A blow from outside
## the target's front, or on a turning squad, is a flank blow. Blows land together. Units
## turn to their foes at their turn rate as they move (FormationScrum, UnitMotion), so a
## unit fighting one foe is flanked by a second, and one turning away is struck from behind.
## Routing squads are left to FormationRout. Pure over the squads it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const FormationCombat = preload("res://sim/skirmish/formation/formation_combat.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const FoeIndex = preload("res://sim/skirmish/formation/foe_index.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## This tick's blows: [[attacker, target, damage, flank], ...]. Updates each striker's
## target and cooldown.
static func blows(squads: Array, interval: int, fight_seed: int) -> Array:
	var out := []
	if squads.is_empty():
		return out
	var index := FoeIndex.of(squads)  # one grid a faction, as they stand
	var bearings: PackedFloat64Array = squads[0].store.columns.column(UnitColumns.BEARING)
	var cooldowns: PackedInt32Array = squads[0].store.columns.column(UnitColumns.ATTACK_COOLDOWN)
	for squad in squads:
		var near := FoeIndex.struck(index, squad)
		if near.is_empty():
			continue
		var reach := BattleTuning.current().reach_contact
		for unit in squad.living():
			var where := ScrumReach.at(squad, unit)
			var close := ScrumNear.around(near, where, ScrumReach.radius(unit) + reach)
			var pick := _pick(squad, unit, bearings[unit.slot], close, fight_seed)
			if pick.is_empty():
				continue
			var target: SkirmishUnit = pick[0]
			var target_at: Vector2 = pick[2]
			unit.target_id = target.id
			cooldowns[unit.slot] -= 1
			if cooldowns[unit.slot] > 0:
				continue
			cooldowns[unit.slot] = FormationMorale.interval(squad, ticks(interval, unit))
			var target_squad: SkirmishSquad = pick[1]
			if (
				squad.order == SkirmishUnit.Order.RETREAT
				and not ScrumReach.in_front(bearings[unit.slot], where, target_at)
			):
				continue  # turned away: it no longer strikes
			var flank := not ScrumReach.in_front(bearings[target.slot], target_at, where)
			out.append([unit, target, FormationCombat.damage(unit), flank])
	squads[0].store.columns.commit(UnitColumns.ATTACK_COOLDOWN, cooldowns)
	return out


## True if the unit touches a living unit of any of `foes` ([[unit, squad], ...]).
static func touches_any(squad: SkirmishSquad, unit: SkirmishUnit, foes: Array) -> bool:
	for entry in foes:
		if ScrumReach.touching(squad, unit, entry[1], entry[0]):
			return true
	return false


## [[unit, squad], ...] for the living units of squads hostile to `squad`, not routing.
static func hostile_units(squad: SkirmishSquad, squads: Array) -> Array:
	return _hostile(squad, squads)


static func _hostile(squad: SkirmishSquad, squads: Array) -> Array:
	var out := []
	for other in squads:
		if other.faction_id == squad.faction_id:
			continue
		if other.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			continue
		for unit in other.living():
			out.append([unit, other])
	return out


## [target, its squad, where it stands] for the enemy the unit facing `facing` strikes; [] if
## it touches none.
static func _pick(
	squad: SkirmishSquad, unit: SkirmishUnit, facing: float, foes: Array, fight_seed: int
) -> Array:
	var where := ScrumReach.at(squad, unit)
	var best := []
	var best_key := []  # [front, distance]; the draw is weighed only on a tie of the two
	for entry in foes:
		var other: SkirmishUnit = entry[0]
		if not ScrumReach.touching(squad, unit, entry[1], other):
			continue
		var there := ScrumReach.at(entry[1], other)
		var front := 0 if ScrumReach.in_front(facing, where, there) else 1
		var key := [front, where.distance_to(there)]
		if (
			best.is_empty()
			or key < best_key
			or (not best_key < key and _drawn_first(other, best[0], fight_seed))
		):
			best = [other, entry[1], there]
			best_key = key
	return best


## True if `unit`'s draw goes before `other`'s: the last of _pick's key.
static func _drawn_first(unit: SkirmishUnit, other: SkirmishUnit, fight_seed: int) -> bool:
	return ScrumContest.draw(unit, fight_seed) < ScrumContest.draw(other, fight_seed)


## Where the nearest enemy the unit touches stands, or null if it touches none.
static func nearest_touching(
	squad: SkirmishSquad, unit: SkirmishUnit, foes: Array, fight_seed: int
):
	var facing: float = squad.store.columns.value(unit.slot, UnitColumns.BEARING)
	var pick := _pick(squad, unit, facing, foes, fight_seed)
	return null if pick.is_empty() else pick[2]


## Ticks between the unit's melee blows, `second` ticks making a second (Decision 120).
static func ticks(second: int, unit: SkirmishUnit) -> int:
	return maxi(1, roundi(second * unit.melee_seconds))
