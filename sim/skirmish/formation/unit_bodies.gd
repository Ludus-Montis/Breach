class_name UnitBodies
extends RefCounted
## Bodies (Decision 106, spec 30 round 1): a unit's body is the circle inscribed in its
## footprint - a grem 1 cell across, a brute 2 - and no two bodies overlap. After every
## move each tick, each pair whose bodies overlap is pushed apart the shortest way:
## - **Friends** split the push by mass (footprint area): a brute moves a grem more than a
##   grem moves a brute. A unit in its formation's frame resists, bodies_resist times its mass,
##   but gives way: pushed, it leaves its place and walks back to it (its squad re-forms).
##   To another squad's loose unit it doesn't give way - a squad's space is kept by its
##   units in control of it - so the loose one takes the whole push; nor to its own
##   squad's when only brushed (under bodies_brush deep). Routers in flight still push through,
##   and its own squad's loose units still shove their way through it.
## - **Foes** are not pushed (Decision 105): two that overlap - one shoved into the other by
##   its friends - each step back half the overlap, so no one is pushed into an enemy.
## - **Two units both in their frames:** of two squads, they are kept apart by the frames
##   (friends queueing between squads: Decision 84; foes halting at contact). Of one squad,
##   nothing else keeps them apart once they walk to their places (they can cross, or close
##   up in a queue at a gap), so they part as friends do, but each stays in its frame and
##   walks on from where it was pushed: only a push from a body out of its frame takes a
##   unit out of its own.
## Pushes are worked out from one snapshot, then applied together, bodies_passes times a tick; a
## pair whose bodies lie exactly on each other parts along an angle seeded by the battle
## and the two units, never by a world direction or the list (Decision 97). Pure over the
## squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const BodyParting = preload("res://sim/skirmish/formation/body_parting.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

## Cells a bucket of the pair search spans: at least the widest body.
const BUCKET := 2.0
## Overlaps shallower than this (cells) are left: Vector2's float32 rounding, not a push.
const EPSILON := 0.001

## Microseconds spent in step() since it was last zeroed (the bench reads it; no outcome does).
static var clock_usec := 0


## The body's radius in cells.
static func radius(unit: SkirmishUnit) -> float:
	return ScrumReach.radius(unit)


## Where the unit's body stands: its point in the scrum or in flight, or its place.
static func at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	if squad.fleeing.has(unit.id):
		return FormationRout.where(squad, unit.id)
	if squad.loose.has(unit.id):
		return SquadEntries.loose_at(squad, unit)
	return squad.store.columns.value(unit.slot, UnitColumns.POSITION)


## Pushes apart every pair of bodies that overlap: in GDScript (the reference) or, when the
## switch picks Rust and it is built, on the battle's native `field` (a BodyField kept
## across ticks; without one, a field made for this step alone).
static func step(squads: Array, fight_seed: int, field: Object = null) -> void:
	var began := Time.get_ticks_usec()
	if field == null and not squads.is_empty():
		field = squads[0].store.field()  # a native pass runs on its store's field
	if field != null:
		var way := func(draw: int, other_draw: int) -> Vector2:
			return part_way(fight_seed, draw, other_draw)
		BodyParting.step(field, squads, fight_seed, way, NativeKernels.threaded)
	else:
		_passes(_drawn(squads, fight_seed), fight_seed)
	clock_usec += Time.get_ticks_usec() - began


## The way two bodies lying exactly on each other part: seeded by the battle and their draws.
static func part_way(fight_seed: int, draw: int, other_draw: int) -> Vector2:
	var angle := TAU * BattleRolls.uniform(fight_seed, [draw, other_draw, "part"])
	return DetMath.rotated(Vector2.RIGHT, angle)


## The GDScript passes: pairs found and pushed from one snapshot, then applied together.
static func _passes(drawn: Array, fight_seed: int) -> void:
	for _pass in range(BattleTuning.current().bodies_passes):
		var bodies := _bodies(drawn)
		var moves := {}  # body index -> [how far it is pushed, by one out of its frame]
		for pair in _pairs(bodies):
			_push(bodies, pair, fight_seed, moves)
		if moves.is_empty():
			return
		for index in moves:
			_move(bodies[index], moves[index][0], moves[index][1])


## [[squad, unit, its draw], ...] for every living unit of a standing squad, in the order
## of their draws (not the list's): the same all through a step.
static func _drawn(squads: Array, fight_seed: int) -> Array:
	var out := []
	for squad in squads:
		if squad.state == SkirmishSquad.State.DESTROYED:
			continue
		for unit in squad.living():
			out.append([squad, unit, ScrumContest.draw(unit, fight_seed)])
	out.sort_custom(func(a, b): return a[2] < b[2])
	return out


## [[squad, unit, where, its draw], ...]: the drawn bodies (_drawn) where they stand now.
static func _bodies(drawn: Array) -> Array:
	var out := []
	for entry in drawn:
		out.append([entry[0], entry[1], at(entry[0], entry[1]), entry[2]])
	return out


## [[i, j], ...] in order, for bodies that overlap, i < j, not in two squads' frames, found
## through buckets.
static func _pairs(bodies: Array) -> Array:
	var count := bodies.size()
	var buckets := {}
	var homes := []
	var framed := []
	var radii := []
	for index in range(count):
		var key := Vector2i((bodies[index][2] / BUCKET).floor())
		homes.append(key)
		framed.append(_framed(bodies[index]))
		radii.append(radius(bodies[index][1]))
		if not buckets.has(key):
			buckets[key] = []
		buckets[key].append(index)
	var found := PackedInt64Array()
	for index in range(count):
		for dy in range(-1, 2):
			for dx in range(-1, 2):
				for other in buckets.get(homes[index] + Vector2i(dx, dy), []):
					if other <= index:
						continue
					if framed[index] and framed[other] and bodies[index][0] != bodies[other][0]:
						continue  # two squads' frames keep them apart
					var apart: Vector2 = bodies[other][2] - bodies[index][2]
					if radii[index] + radii[other] - apart.length() > EPSILON:  # as _push weighs it
						found.append(index * count + other)
	found.sort()
	var out := []
	for pair in found:
		out.append([pair / count, pair % count])
	return out


## True if the unit stands in its formation's frame (neither loose nor fleeing).
static func _framed(body: Array) -> bool:
	return not body[0].loose.has(body[1].id) and not body[0].fleeing.has(body[1].id)


## Adds the pair's push to `moves`: each body out along the line between them - friends
## by the other's share of their mass, foes half each - noting a push from a body out of
## its frame (it takes one in its frame out of it).
static func _push(bodies: Array, pair: Array, fight_seed: int, moves: Dictionary) -> void:
	var a: Array = bodies[pair[0]]
	var b: Array = bodies[pair[1]]
	var apart: Vector2 = b[2] - a[2]
	var depth := radius(a[1]) + radius(b[1]) - apart.length()
	if depth <= EPSILON:
		return
	var way := apart.normalized()
	if apart.length() < EPSILON:
		way = part_way(fight_seed, a[3], b[3])
	var share := 0.5
	if a[0].faction_id == b[0].faction_id:
		share = _mass(b) / (_mass(a) + _mass(b))
		if _keeps(a, b, depth):
			share = 0.0
		elif _keeps(b, a, depth):
			share = 1.0
	var loosens := not (_framed(a) and _framed(b))
	if share > 0.0:  # a body that holds its space isn't moved, so stays in its frame
		var held: Array = moves.get(pair[0], [Vector2.ZERO, false])
		moves[pair[0]] = [held[0] - way * depth * share, held[1] or loosens]
	if share < 1.0:
		var held: Array = moves.get(pair[1], [Vector2.ZERO, false])
		moves[pair[1]] = [held[0] + way * depth * (1.0 - share), held[1] or loosens]


## True if `keeper`, in its frame, holds its space against `other`, a loose unit: always
## against another squad's, and against its own squad's when only brushed (one in flight,
## out of control, still pushes through).
static func _keeps(keeper: Array, other: Array, depth: float) -> bool:
	if not _framed(keeper) or not other[0].loose.has(other[1].id):
		return false
	return keeper[0] != other[0] or depth < BattleTuning.current().bodies_brush


## What a body weighs against a push: its footprint's area, bodies_resist times over in its frame.
static func _mass(body: Array) -> float:
	var unit: SkirmishUnit = body[1]
	var area := float(unit.footprint_width * unit.footprint_depth)
	return area * BattleTuning.current().bodies_resist if _framed(body) else area


## Moves the body by `push`: a router's flight, a loose unit's point, or - for one in its
## frame - out of its place, to walk back to it, if a body out of its frame pushed it
## (`loosens`), else where it stands, walking on in its frame.
static func _move(body: Array, push: Vector2, loosens: bool) -> void:
	var squad: SkirmishSquad = body[0]
	var unit: SkirmishUnit = body[1]
	var columns := squad.store.columns
	if squad.fleeing.has(unit.id):
		var offset: Vector2 = columns.value(unit.slot, UnitColumns.FLEE_OFFSET)
		# placement: parting clears an overlap (Decision 139)
		columns.put(unit.slot, UnitColumns.FLEE_OFFSET, offset + push)
		return
	if not loosens and not squad.loose.has(unit.id):
		# placement: parting clears an overlap (Decision 139)
		columns.put(unit.slot, UnitColumns.POSITION, body[2] + push)
		return
	if not squad.loose.has(unit.id):
		SquadEntries.loosen(squad, unit, body[2])
	var at: Vector2 = SquadEntries.loose_at(squad, unit)
	var resting: bool = columns.value(unit.slot, UnitColumns.NEXT) == at
	# placement: parting clears an overlap (Decision 139)
	columns.put(unit.slot, UnitColumns.LOOSE_AT, at + push)
	if resting:
		columns.put(unit.slot, UnitColumns.NEXT, at + push)
