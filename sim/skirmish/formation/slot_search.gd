class_name SlotSearch
extends RefCounted
## Finding a slot in the scrum without looking at every one (spec 30 round 3): the same
## choice as ScrumSlots makes - the nearest open slot, equals going by its key - but the
## slots round a squad's foes are bucketed once a tick (BodyGrid), with the bodies standing
## on each, and a seeker looks ring by ring outwards from where it stands, stopping once no
## ring further out could hold a nearer one. Claims on a slot are looked up the same way. A
## tie the key leaves goes to the slot listed first, as it would in the whole list
## (Decision 97). Whether a slot is in a seeker's leash and on ground it can cross is
## ScrumSlots' own test (its _within), used as it is so the two never drift. Pure; cells.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")

## Cells added to every bound: far above float rounding, so no slot is missed.
const MARGIN := 0.01


## The slots round the foes ([[unit, squad], ...]) for a seeker of `radius`, indexed:
## {"slots": as ScrumSlots.round_foes, "grid": over all of them, "by_foe": foe id ->
## [first slot, count], "free": the slots no body stands on, "free_grid" over those,
## "own": unit -> the slots only its own body stands on, and "dead": the slots found
## claimed}. `crowd` is a BodyGrid.of_bodies
## over `bodies` (ScrumSlots.bodies), as they stand all through the seeking.
static func ring(foes: Array, radius: float, bodies: Array, crowd: Dictionary) -> Dictionary:
	var slots := ScrumSlots.round_foes(foes, radius)
	var by_foe := {}
	var free := []
	var own := {}
	for index in range(slots.size()):
		var foe: SkirmishUnit = slots[index][1]
		if not by_foe.has(foe.id):
			by_foe[foe.id] = [index, 0]
		by_foe[foe.id][1] += 1
		var on := _standing_on(slots[index][0], radius, bodies, crowd)
		if on.is_empty():
			free.append(index)
		elif on.size() == 1:
			if not own.has(on[0]):
				own[on[0]] = []
			own[on[0]].append(index)
	var points := slots.map(func(slot): return slot[0])
	return {
		"slots": slots,
		"grid": BodyGrid.build(points),
		"by_foe": by_foe,
		"free": free,
		"free_grid": BodyGrid.build(free.map(func(index): return points[index])),
		"own": own,
		"dead": {},  # slots found claimed: claims only grow while seekers pick
		"gone": 0,  # of them, how many have left the free grid
	}


## The slot `goal` ([foe id, slot number]) names in the ring, or [].
static func find(slot_ring: Dictionary, goal: Array) -> Array:
	var span = slot_ring["by_foe"].get(goal[0])
	if span == null or goal[1] < 0 or goal[1] >= span[1]:
		return []
	var slot: Array = slot_ring["slots"][span[0] + goal[1]]
	return slot if slot[3] == goal[1] else []


## As ScrumSlots.pick, over the ring: the slot the seeker at `at` facing `facing` makes for,
## or []. Open
## slots are the free ones and its own, with no claim near; `ground` is ScrumSlots' with
## [5] a BodyGrid.of_bodies over its bodies and [6] the claims' cells (claim()).
static func pick(
	seeker: SkirmishUnit,
	at: Vector2,
	facing: float,
	slot_ring: Dictionary,
	ground: Array,
	crowded := false
) -> Array:
	var search := {
		"seeker": seeker,
		"at": at,
		"ahead": UnitMotion.vector(facing),
		"ground": ground,
		"crowded": crowded,
		"slots": slot_ring["slots"],
		"ring": slot_ring,
		"dead": slot_ring["dead"],
		"best": -1,
		"key": [],
	}
	if crowded:
		_outwards(search, slot_ring["grid"], [])
	else:
		for index in slot_ring["own"].get(seeker, []):
			_consider(search, index)
		_refresh(slot_ring)
		if not slot_ring["free"].is_empty():
			_outwards(search, slot_ring["free_grid"], slot_ring["free"])
	return [] if search["best"] < 0 else search["slots"][search["best"]]


## Remakes the ring's free grid without the slots found claimed, once they are half of it.
static func _refresh(slot_ring: Dictionary) -> void:
	var free: Array = slot_ring["free"]
	if slot_ring["gone"] * 2 <= free.size():
		return
	var slots: Array = slot_ring["slots"]
	var dead: Dictionary = slot_ring["dead"]
	free = free.filter(func(index): return not dead.has(index))
	slot_ring["free"] = free
	slot_ring["free_grid"] = BodyGrid.build(free.map(func(index): return slots[index][0]))
	slot_ring["gone"] = 0


## As ScrumSlots.open, looking only at the bodies and claims near `point`.
static func open(seeker: SkirmishUnit, point: Vector2, ground: Array) -> bool:
	if not ScrumSlots._within(seeker, point, ground):
		return false
	var radius := ScrumReach.radius(seeker)
	var bodies: Array = ground[1]
	var crowd: Dictionary = ground[5]
	for index in BodyGrid.near(crowd, point, crowd["widest"] + radius + MARGIN):
		var body: Array = bodies[index]
		if body[2] != seeker and body[0].distance_to(point) < body[1] + radius - ScrumSlots.EPSILON:
			return false
	return not _claimed(seeker, point, ground[6])


## Adds a claimed point to the claims' cells ({cell: [point, ...]}).
static func claim(claims: Dictionary, point: Vector2) -> void:
	var cell := BodyGrid.cell_of(point)
	if not claims.has(cell):
		claims[cell] = []
	claims[cell].append(point)


## The units whose bodies stand on `point` for a seeker of `radius`, as ScrumSlots.open
## sees them; it stops at two.
static func _standing_on(point: Vector2, radius: float, bodies: Array, crowd: Dictionary) -> Array:
	var on := []
	var cells: Dictionary = crowd["cells"]
	var reach: float = crowd["widest"] + radius + MARGIN
	var from := BodyGrid.cell_of(point - Vector2(reach, reach))
	var to := BodyGrid.cell_of(point + Vector2(reach, reach))
	for y in range(from.y, to.y + 1):
		for x in range(from.x, to.x + 1):
			for index in cells.get(Vector2i(x, y), []):
				var body: Array = bodies[index]
				if body[0].distance_to(point) < body[1] + radius - ScrumSlots.EPSILON:
					on.append(body[2])
					if on.size() == 2:
						return on
	return on


## Considers the slots of `grid` ring by ring out from the seeker, until no ring further
## out could hold one nearer than the best, or one within its leash; `listed` maps the
## grid's points to slots (empty: the same). A free slot found claimed leaves the grid.
static func _outwards(search: Dictionary, grid: Dictionary, listed: Array) -> void:
	var at: Vector2 = search["at"]
	var centre := BodyGrid.cell_of(at)
	var ground: Array = search["ground"]
	var tether: float = BattleTuning.current().scrum_leash  # from its place (ScrumSlots.within)
	var leash: float = tether + at.distance_to(ground[0])
	var slots: Array = search["slots"]
	var span := BodyGrid.ring_span(grid, centre, ground[0], tether)
	for ring_number in range(span.x, span.y + 1):
		var nearest := BodyGrid.floor_of(ring_number) - MARGIN
		if nearest > leash or (search["best"] >= 0 and nearest > search["key"][0]):
			return
		var around: Vector2 = at if search["best"] >= 0 else ground[0]  # near enough to beat
		var reach: float = search["key"][0] if search["best"] >= 0 else tether  # it, or leashed
		for found in BodyGrid.ring(grid, centre, ring_number, around, reach):
			var index: int = found if listed.is_empty() else listed[found]
			var point: Vector2 = slots[index][0]
			if search["best"] >= 0 and snappedf(at.distance_to(point), 0.000001) > search["key"][0]:
				continue  # farther than the best (as _consider weighs it)
			_consider(search, index)
			if not search["crowded"] and search["dead"].has(index):  # claimed: gone for all
				grid["cells"][BodyGrid.cell_of(point)].erase(found)
				search["ring"]["gone"] += 1


## Weighs one slot: it becomes the best if it qualifies and its key (ScrumSlots.pick) is
## lower, or the same and it is listed first.
static func _consider(search: Dictionary, index: int) -> void:
	var slot: Array = search["slots"][index]
	var at: Vector2 = search["at"]
	var distance := snappedf(at.distance_to(slot[0]), 0.000001)
	var best: int = search["best"]
	if best >= 0 and distance > search["key"][0]:
		return
	if not search["crowded"] and search["dead"].has(index):
		return
	var seeker: SkirmishUnit = search["seeker"]
	var ground: Array = search["ground"]
	if not ScrumSlots._within(seeker, slot[0], ground):
		return
	if not search["crowded"] and _claimed(seeker, slot[0], ground[6]):
		search["dead"][index] = true
		return
	var key := [
		distance,
		-snappedf((slot[0] - at).dot(search["ahead"]), 0.000001),
		snappedf(slot[0].distance_to(ground[0]), 0.000001),
	]
	if best >= 0 and not key < search["key"]:
		if search["key"] < key or not _draws_first(search, index, best):
			return
	search["best"] = index
	search["key"] = key


## True if slot `index` goes before slot `best` on the last of their keys - the foe's draw,
## then the seeker's roll for the slot, then the slot listed first - the rest being equal.
## The draws are worked out only for such a tie.
static func _draws_first(search: Dictionary, index: int, best: int) -> bool:
	var mine := _draws(search, search["slots"][index])
	var theirs := _draws(search, search["slots"][best])
	return mine < theirs or (mine == theirs and index < best)


## [the foe's draw, the seeker's roll] for the slot: the tail of _consider's key.
static func _draws(search: Dictionary, slot: Array) -> Array:
	var draw_seed: int = search["ground"][4]
	return [
		ScrumContest.draw(slot[1], draw_seed),
		BattleRolls.uniform(draw_seed, [search["seeker"].id, slot[1].id, slot[3], "slot"]),
	]


## True if a claim (claim()) lies within a body's breadth of `point`.
static func _claimed(seeker: SkirmishUnit, point: Vector2, claims: Dictionary) -> bool:
	var radius := ScrumReach.radius(seeker)
	var from := BodyGrid.cell_of(point - Vector2.ONE * (2.0 * radius + MARGIN))
	var to := BodyGrid.cell_of(point + Vector2.ONE * (2.0 * radius + MARGIN))
	for y in range(from.y, to.y + 1):
		for x in range(from.x, to.x + 1):
			for claim_point in claims.get(Vector2i(x, y), []):
				if claim_point.distance_to(point) < 2.0 * radius - ScrumSlots.EPSILON:
					return true
	return false
