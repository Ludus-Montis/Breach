class_name FormationPour
extends RefCounted
## Pouring through a gap (spec 30 round 3, part 2): a unit whose place lies on ground it
## can't walk makes for the nearest point across its frame it can (FormationWalk.target_of),
## and where several such points fall on one another, or on a place a unit already stands
## for, they file in behind it: each squeezed unit, front first, then nearest its frame's
## centre line, then by its seeded draw (Decision 97), takes the first point back from its
## own - a body's breadth at a time, against its formation's way - that it can walk and that
## no target taken before holds. So a formation queues at a ford and pours through it
## rather than packing its units onto one point. Pure; cells.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001
## Cells a unit squeezes in at a time, looking for ground to stand on.
const SQUEEZE_STEP := 0.5


## Where `unit` belongs in its squad's frame now, standing at `place` (rank, column),
## including any swap under way (`moving`: FormationShuffle.movers for the squad, or null).
static func place(squad: SkirmishSquad, unit: SkirmishUnit, moving, place: Vector2i) -> Vector2:
	var swapping := FormationShuffle.offset(squad, unit, moving)
	var ahead := UnitMotion.vector(squad.heading)
	var shift := ahead * swapping.x - ahead.orthogonal() * swapping.y
	var frame := [squad.position, squad.heading, squad.width, squad.centre_shift]
	var at := SquadFrame.place(frame[0], frame[1], frame[2], frame[3], unit, place)
	return at + shift


## unit id -> where each of the squad's units in its frame (and its loose ones, if
## `with_loose`) makes for: one out taking a
## downed foe, beside it; another, its place, or - where its place lies on ground it can't
## walk - the nearest it can walk, squeezing across the frame towards its centre line (so a
## formation pours through a gap narrower than itself and fans out again past it, and wades
## a ford rather than swim beside it: spec 30 round 3, parts 5 and 6), the squeezed filing
## in behind one another (_queue); with none to walk, its place if it can swim or
## climb there, else the nearest it can stand on.
static func targets(
	squad: SkirmishSquad,
	terrain: FormationTerrain = null,
	moving = null,
	fight_seed: int = 0,
	with_loose := false
) -> Dictionary:
	var aims := []  # [unit, its place, its point, squeezed]
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	var places := UnitView.read(squad, UnitView.PLACES)  # read once a pass (Decision 146)
	for unit in squad.living():
		if squad.fleeing.has(unit.id) or (squad.loose.has(unit.id) and not with_loose):
			continue
		if squad.taking.has(unit.id):  # out taking a downed foe (FormationTaking): beside it
			var body: SkirmishUnit = squad.taking[unit.id]
			aims.append([unit, at[unit.slot], _beside(at[unit.slot], at[body.slot]), false])
			continue
		var own := place(squad, unit, moving, UnitPlace.in_view(places, unit))
		var aim := _aim(squad, unit, own, terrain)
		aims.append([unit, own, aim, not aim.is_equal_approx(own)])
	return _queue(squad, aims, terrain, fight_seed)


## Beside the downed foe lying at `body` that the unit at `unit` is out taking.
static func _beside(unit: Vector2, body: Vector2) -> Vector2:
	var off := unit - body
	var beside := minf(off.length(), BattleTuning.current().wounds_reach * 0.5)
	return body + (off.normalized() * beside if off.length() > EPSILON else off)


## The unit's own point, before any files in: its place, or the nearest it can stand on.
static func _aim(
	squad: SkirmishSquad, unit: SkirmishUnit, own: Vector2, terrain: FormationTerrain
) -> Vector2:
	if terrain == null or walkable(unit, own, terrain):
		return own
	var walk := _squeeze(squad, unit, own, terrain, true)  # ground it can walk, near by
	if walk != Vector2.INF:
		return walk
	if terrain.factor(unit, own, own) > 0.0:
		return own  # none to walk: it swims or climbs to its place
	var any := _squeeze(squad, unit, own, terrain, false)
	return own if any == Vector2.INF else any


## The nearest point across the frame from `own` (its place) towards its centre line where
## `unit` can stand - walking it, if `walking` - or INF.
static func _squeeze(
	squad: SkirmishSquad, unit: SkirmishUnit, own: Vector2, terrain: FormationTerrain, walking: bool
) -> Vector2:
	var across := UnitMotion.vector(squad.heading).orthogonal()
	var offset := (own - squad.position).dot(across)
	var steps := ceili(absf(offset) / SQUEEZE_STEP)
	for step in range(1, steps + 1):
		var point := own - across * signf(offset) * minf(step * SQUEEZE_STEP, absf(offset))
		var ok := (
			walkable(unit, point, terrain) if walking else terrain.factor(unit, point, point) > 0.0
		)
		if ok:
			return point
	return Vector2.INF


## unit id -> where it makes for. `aims` = [[unit, its place, its point, squeezed], ...]:
## the unsqueezed keep their points; the squeezed file in behind theirs, in turn.
static func _queue(
	squad: SkirmishSquad, aims: Array, terrain: FormationTerrain, fight_seed: int
) -> Dictionary:
	var out := {}
	var ahead := UnitMotion.vector(squad.heading)
	var turns := []  # [[front first, nearest the centre line, draw], aim]
	for aim in aims:
		out[aim[0].id] = aim[2]
		if not aim[3]:
			continue
		var off: Vector2 = aim[1] - squad.position
		var key := [-snappedf(off.dot(ahead), EPSILON)]
		key.append(snappedf(absf(off.dot(ahead.orthogonal())), EPSILON))
		key.append(ScrumContest.draw(aim[0], fight_seed))
		turns.append([key, aim])
	if turns.is_empty():
		return out  # nobody squeezed: every unit makes for its own place
	var taken := _taken(aims)
	turns.sort_custom(func(a, b): return a[0] < b[0])
	for turn in turns:
		var aim: Array = turn[1]
		var point := _file_in(aim[0], aim[2], ahead, taken, [terrain, aims.size()])
		out[aim[0].id] = point
		_take(taken, point, ScrumReach.radius(aim[0]))
	return out


## The targets the unsqueezed hold, bucketed by where they lie.
static func _taken(aims: Array) -> Dictionary:
	var widest := 0.5
	for aim in aims:
		widest = maxf(widest, ScrumReach.radius(aim[0]))
	var taken := {"bucket": widest * 2.0, "cells": {}, "ends": {}}
	for aim in aims:
		if not aim[3]:
			_take(taken, aim[2], ScrumReach.radius(aim[0]))
	return taken


## The first point from `point` back against `ahead`, a body's breadth at a time, that the
## unit can walk and no target taken holds; `point` if none is within `limits` = [terrain,
## how many steps back it looks]. A queue is taken up where its last unit filed in, so a
## long one costs a step a unit.
static func _file_in(
	unit: SkirmishUnit, point: Vector2, ahead: Vector2, taken: Dictionary, limits: Array
) -> Vector2:
	var breadth := ScrumReach.radius(unit) * 2.0
	var queue_key := [point, breadth]  # where the last to file in here stood: none nearer
	for step in range(taken["ends"].get(queue_key, 0), limits[1] + 1):
		var at := point - ahead * (breadth * step)
		if (step == 0 or walkable(unit, at, limits[0])) and _free(taken, at, unit):
			taken["ends"][queue_key] = step
			return at
	return point


## True if no target taken lies within the unit's body and its own of `at`.
static func _free(taken: Dictionary, at: Vector2, unit: SkirmishUnit) -> bool:
	var home := Vector2i((at / taken["bucket"]).floor())
	var own := ScrumReach.radius(unit)
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			for held in taken["cells"].get(home + Vector2i(dx, dy), []):
				if at.distance_to(held[0]) < own + held[1] - EPSILON:
					return false
	return true


static func _take(taken: Dictionary, at: Vector2, radius: float) -> void:
	var home := Vector2i((at / taken["bucket"]).floor())
	if not taken["cells"].has(home):
		taken["cells"][home] = []
	taken["cells"][home].append([at, radius])


## Whether `unit` can walk on the point - not swim or climb to it.
static func walkable(unit: SkirmishUnit, point: Vector2, terrain: FormationTerrain) -> bool:
	if terrain == null:
		return true
	var walker := TerrainWalker.of_unit(unit)
	return (
		terrain.factor(unit, point, point) > 0.0
		and terrain.mode(walker, point, point) == TerrainWalker.Mode.WALKING
	)
