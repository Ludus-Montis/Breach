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
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001


## This tick's ranged strikes: [[shooter, target, damage, shooter's squad], ...]. Updates
## each shooter's target and cooldown, like melee fighters.
static func ranged_blows(squads: Array, interval_ticks: int, fight_seed: int = 0) -> Array:
	var blows := []
	if squads.is_empty():
		return blows
	var views := {}  # store -> its places, read once a pass (Decision 150): nothing here moves
	var marks := _marks(squads, views)
	var cooldowns: PackedInt32Array = squads[0].store.columns.column(UnitColumns.ATTACK_COOLDOWN)
	var targets: PackedInt64Array = squads[0].store.columns.column(UnitColumns.TARGET_ID)
	for own in squads:
		if own.is_destroyed() or own.state == SkirmishSquad.State.ARRIVED:
			continue
		var melee := {}
		if own.state == SkirmishSquad.State.FIGHTING:
			for fighter in own.fighters():
				melee[fighter] = true
		var rows := []  # [how far each rank of a hostile squad stands ahead of own's front]
		var places: UnitView = null  # own's places, taken once it has a shooter
		for shooter in own.living():
			if shooter.attack_range <= 0 or melee.has(shooter):
				continue
			if rows.is_empty():
				rows = _rows(own, marks)
				places = UnitView.read_once(views, own, UnitView.PLACES)
			var target := _in_range(shooter, rows, [fight_seed, places])
			if target == null:
				continue
			targets[shooter.slot] = target.id
			cooldowns[shooter.slot] -= 1
			if cooldowns[shooter.slot] <= 0:
				cooldowns[shooter.slot] = maxi(1, roundi(interval_ticks * shooter.ranged_seconds))
				blows.append([shooter, target, shooter.ranged_dmg, own])
	squads[0].store.columns.commit(UnitColumns.ATTACK_COOLDOWN, cooldowns)
	squads[0].store.columns.commit(UnitColumns.TARGET_ID, targets)
	return blows


## A melee blow's damage before it lands: all the striker's melee weapons together. How
## it lands - a flank finding no parry - is BlowLanding's (Decision 118).
static func damage(fighter: SkirmishUnit) -> int:
	return fighter.dmg


## The squads a shot may strike, as [squad, {rank: [[unit, its place in the listing],
## ...]}], in the order they are listed: a unit's gap from a shooter goes only by its rank.
static func _marks(squads: Array, views: Dictionary) -> Array:
	var out := []
	var place := 0
	for other in squads:
		if other.state == SkirmishSquad.State.ARRIVED:
			continue
		var ranks := {}
		var places := UnitView.read_once(views, other, UnitView.PLACES)
		var other_ranks: PackedInt32Array = places.rank
		for unit in other.living():
			var rank: int = other_ranks[unit.slot]
			if not ranks.has(rank):
				ranks[rank] = []
			ranks[rank].append([unit, place])
			place += 1
		if not ranks.is_empty():
			out.append([other, ranks, places])
	return out


## Own's frame (StoredSquad.frame, read once a pass: Decision 144), then for each squad hostile
## to `own`: [squad, [[how far the rank stands ahead of own's front (SquadGeometry.unit_gap),
## its units], ...], its places (a view of UnitView.PLACES), its frame].
static func _rows(own: SkirmishSquad, marks: Array) -> Array:
	var mine := own.frame()
	var out := [mine]  # never empty, so worked out once a squad
	for mark in marks:
		var other: SkirmishSquad = mark[0]
		if other.faction_id == own.faction_id:
			continue
		var theirs := other.frame()
		var ranks := []
		for rank in mark[1]:
			var row: Array = mark[1][rank]
			ranks.append([SquadGeometry.unit_gap(mine, theirs, row[0][0], mark[2]), row])
		out.append([other, ranks, mark[2], theirs])
	return out


## The nearest enemy unit within the shooter's range (ties: one it overlaps laterally,
## then the targets' draws, then the order they are listed in). Only the ranks at the
## nearest gap are looked at unit by unit: all a rank's units share its gap. `drawn`: [the
## fight seed, own's places (a view of UnitView.PLACES)]; `rows`: _rows', own's frame first.
static func _in_range(shooter: SkirmishUnit, rows: Array, drawn: Array) -> SkirmishUnit:
	var fight_seed: int = drawn[0]
	var places: UnitView = drawn[1]
	var mine: Array = rows[0]
	var reach := shooter.attack_range * SkirmishSquad.RANK_DEPTH + EPSILON
	var here := SquadGeometry.unit_gap(mine, mine, shooter, places)
	var nearest := INF
	for index in range(1, rows.size()):
		for rank in rows[index][1]:
			var gap: float = absf(rank[0] - here)
			if gap <= reach:
				nearest = minf(nearest, snappedf(gap, EPSILON))
	if nearest == INF:
		return null
	var axis := SquadFrame.lateral_axis(mine[1])
	var span := SquadFrame.span(mine, shooter, Vector2.ZERO, UnitPlace.in_view(places, shooter))
	var best: SkirmishUnit = null
	var best_key := []
	for index in range(1, rows.size()):
		var theirs: Array = rows[index][3]
		for rank in rows[index][1]:
			var gap: float = absf(rank[0] - here)
			if gap > reach or snappedf(gap, EPSILON) != nearest:
				continue
			for entry in rank[1]:
				var target_span := SquadFrame.span(
					theirs, entry[0], axis, UnitPlace.in_view(rows[index][2], entry[0])
				)
				var overlap := minf(span.y, target_span.y) - maxf(span.x, target_span.x)
				var draw := ScrumContest.draw(entry[0], fight_seed)
				var key := [0 if overlap > EPSILON else 1, draw, entry[1]]
				if best == null or key < best_key:
					best = entry[0]
					best_key = key
	return best
