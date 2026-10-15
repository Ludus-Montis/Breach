class_name FormationCombat
extends RefCounted
## How hard a blow lands, and ranged strikes, per specs/22-formation-feel-test.md and
## Decision 40. Pure: FormationSimulation calls it each tick.
##
## Ranged units (Decisions 46-47) strike with their best ranged weapon from anywhere in
## their squad, moving or fighting: the nearest enemy unit within its range in ranks,
## preferring one they overlap laterally, then by the seeded draw (Decision 97). A ranged
## unit in the front rank of an engaged squad strikes with its melee weapons instead.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")

const EPSILON := 0.000001


## This tick's ranged strikes: [[shooter, target, damage, shooter's squad], ...]. Updates
## each shooter's target and cooldown, like melee fighters.
static func ranged_blows(squads: Array, interval_ticks: int, fight_seed: int = 0) -> Array:
	var blows := []
	var marks := _marks(squads)
	for own in squads:
		if own.is_destroyed() or own.state == SkirmishSquad.State.ARRIVED:
			continue
		var melee := {}
		if own.state == SkirmishSquad.State.FIGHTING:
			for fighter in own.fighters():
				melee[fighter] = true
		var rows := []  # [how far each rank of a hostile squad stands ahead of own's front]
		var places := []  # own's ranks and columns, fetched once it has a shooter (spec 35)
		for shooter in own.living():
			if shooter.attack_range <= 0 or melee.has(shooter):
				continue
			if rows.is_empty():
				rows = _rows(own, marks)
				places = UnitPlace.table(own)
			var target := _in_range(own, shooter, rows, [fight_seed, places])
			if target == null:
				continue
			shooter.target_id = target.id
			shooter.attack_cooldown -= 1
			if shooter.attack_cooldown <= 0:
				shooter.attack_cooldown = maxi(1, roundi(interval_ticks * shooter.ranged_seconds))
				blows.append([shooter, target, shooter.ranged_dmg, own])
	return blows


## A melee blow's damage before it lands: all the striker's melee weapons together. How
## it lands - a flank finding no parry - is BlowLanding's (Decision 118).
static func damage(fighter: SkirmishUnit) -> int:
	return fighter.dmg


## The squads a shot may strike, as [squad, {rank: [[unit, its place in the listing],
## ...]}], in the order they are listed: a unit's gap from a shooter goes only by its rank.
static func _marks(squads: Array) -> Array:
	var out := []
	var place := 0
	for other in squads:
		if other.state == SkirmishSquad.State.ARRIVED:
			continue
		var ranks := {}
		var places := UnitPlace.table(other)  # fetched once a squad (spec 35)
		for unit in other.living():
			var rank: int = places[0][unit.slot]
			if not ranks.has(rank):
				ranks[rank] = []
			ranks[rank].append([unit, place])
			place += 1
		if not ranks.is_empty():
			out.append([other, ranks, places])
	return out


## For each squad hostile to `own`: [squad, [[how far the rank stands ahead of own's front
## (SquadGeometry.unit_gap), its units], ...], its places (UnitPlace.table)].
static func _rows(own: SkirmishSquad, marks: Array) -> Array:
	var out := [null]  # never empty, so worked out once a squad
	for mark in marks:
		var other: SkirmishSquad = mark[0]
		if other.faction_id == own.faction_id:
			continue
		var ranks := []
		for rank in mark[1]:
			var row: Array = mark[1][rank]
			ranks.append([SquadGeometry.unit_gap(own, other, row[0][0], mark[2]), row])
		out.append([other, ranks, mark[2]])
	return out


## The nearest enemy unit within the shooter's range (ties: one it overlaps laterally,
## then the targets' draws, then the order they are listed in). Only the ranks at the
## nearest gap are looked at unit by unit: all a rank's units share its gap. `drawn`: [the
## fight seed, own's places (UnitPlace.table)].
static func _in_range(
	own: SkirmishSquad, shooter: SkirmishUnit, rows: Array, drawn: Array
) -> SkirmishUnit:
	var fight_seed: int = drawn[0]
	var places: Array = drawn[1]
	var reach := shooter.attack_range * SkirmishSquad.RANK_DEPTH + EPSILON
	var here := SquadGeometry.unit_gap(own, own, shooter, places)
	var nearest := INF
	for index in range(1, rows.size()):
		for rank in rows[index][1]:
			var gap: float = absf(rank[0] - here)
			if gap <= reach:
				nearest = minf(nearest, snappedf(gap, EPSILON))
	if nearest == INF:
		return null
	var axis := SquadFrame.lateral_axis(own.heading)
	var span := own.lateral_span(shooter, Vector2.ZERO, places)
	var best: SkirmishUnit = null
	var best_key := []
	for index in range(1, rows.size()):
		var other: SkirmishSquad = rows[index][0]
		for rank in rows[index][1]:
			var gap: float = absf(rank[0] - here)
			if gap > reach or snappedf(gap, EPSILON) != nearest:
				continue
			for entry in rank[1]:
				var target_span: Vector2 = other.lateral_span(entry[0], axis, rows[index][2])
				var overlap := minf(span.y, target_span.y) - maxf(span.x, target_span.x)
				var draw := ScrumContest.draw(entry[0], fight_seed)
				var key := [0 if overlap > EPSILON else 1, draw, entry[1]]
				if best == null or key < best_key:
					best = entry[0]
					best_key = key
	return best
