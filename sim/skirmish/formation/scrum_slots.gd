class_name ScrumSlots
extends RefCounted
## Finding a place in the scrum (Decisions 88 and 106, spec 30 round 1): round each foe's
## body, the slots where a seeker's body would touch it - as many as fit round it (6 for a
## grem round a grem, 9 round a brute), laid out from the foe's own front, clockwise. A
## seeker makes for the nearest open slot within the leash of its place (Decision 75): one
## no other body stands on, no seeker nearer in the contest (ScrumContest) has claimed, and
## on ground it can cross. Equally near slots go by the seeker's own frame - the one most
## ahead of it, then the one nearest its place - then by the foes' draws, and two mirror
## images of each other (left and right of it, round one foe) by a draw seeded by the battle,
## the seeker and the slot: no world direction and no handedness is preferred (Decision 97,
## spec 30 agenda 5). It walks straight there, bodies parting round it (UnitBodies). With
## none open it presses in: it waits a body's breadth short of the nearest, taken, slot,
## for the next to open (ScrumSeek, Decision 108), rather than going back to its place.
## Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")

## Tolerance (cells) for bodies just touching: well above Vector2's float32 rounding.
const EPSILON := 0.001


## [[where, radius, unit], ...]: every living unit's body in standing squads.
static func bodies(squads: Array) -> Array:
	var out := []
	var views := {}  # store -> where its units stand, read once (Decision 146)
	for squad in squads:
		if squad.state in [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]:
			continue
		var standing := UnitView.read_once(views, squad, ScrumReach.STANDING)
		for unit in squad.living():
			out.append([ScrumReach.at_in(standing, squad, unit), ScrumReach.radius(unit), unit])
	return out


## [[point, foe unit, foe squad, slot number], ...] round the foes ([[unit, squad], ...])
## for a seeker of body radius `radius`.
static func round_foes(foes: Array, radius: float) -> Array:
	var out := []
	if foes.is_empty():
		return out
	var bearings: PackedFloat64Array = foes[0][1].store.columns.column(UnitColumns.BEARING)
	for entry in foes:
		var foe: SkirmishUnit = entry[0]
		var centre := ScrumReach.at(entry[1], foe)
		var reach := ScrumReach.radius(foe) + radius
		var count := maxi(3, floori(PI * reach / radius + 0.000001))
		for slot in range(count):
			var way := UnitMotion.vector(bearings[foe.slot] + slot * 360.0 / count)
			out.append([centre + way * reach, foe, entry[1], slot])
	return out


## Cells from `at` to the nearest foe body's edge, less the seeker's own radius (0 if it
## touches one); INF with no foes.
static func gap_to(at: Vector2, radius: float, foes: Array) -> float:
	var least := INF
	for entry in foes:
		var edge := ScrumReach.at(entry[1], entry[0]).distance_to(at)
		least = minf(least, edge - ScrumReach.radius(entry[0]) - radius)
	return maxf(least, 0.0)


## The slot (as round_foes gives it) the seeker at `at` facing `facing` makes for, or [] if
## none is open.
## `ground` is [its place, the bodies, the claimed points, the terrain or null, the seed].
## `crowded`: the nearest it could stand on were it free, others' bodies and claims aside
## (one with no open slot waits behind it).
static func pick(
	seeker: SkirmishUnit, at: Vector2, facing: float, slots: Array, ground: Array, crowded := false
) -> Array:
	var ahead := UnitMotion.vector(facing)
	var best := []
	var best_key := []
	for slot in slots:
		if not (_within(seeker, slot[0], ground) if crowded else open(seeker, slot[0], ground)):
			continue
		var key := [
			snappedf(at.distance_to(slot[0]), 0.000001),
			-snappedf((slot[0] - at).dot(ahead), 0.000001),
			snappedf(slot[0].distance_to(ground[0]), 0.000001),
			ScrumContest.draw(slot[1], ground[4]),
			BattleRolls.uniform(ground[4], [seeker.id, slot[1].id, slot[3], "slot"]),
		]
		if best.is_empty() or key < best_key:
			best = slot
			best_key = key
	return best


## True if the seeker may take the slot at `point`: within its leash, on ground it can
## cross, with no other body on it and no claim within a body's breadth of it.
static func open(seeker: SkirmishUnit, point: Vector2, ground: Array) -> bool:
	if not _within(seeker, point, ground):
		return false
	var radius := ScrumReach.radius(seeker)
	for body in ground[1]:
		if body[2] != seeker and body[0].distance_to(point) < body[1] + radius - EPSILON:
			return false
	for claim in ground[2]:
		if claim.distance_to(point) < 2.0 * radius - EPSILON:
			return false
	return true


## True if `point` is within the seeker's leash, on ground it can cross.
static func _within(seeker: SkirmishUnit, point: Vector2, ground: Array) -> bool:
	if point.distance_to(ground[0]) > BattleTuning.current().scrum_leash:
		return false
	var terrain: FormationTerrain = ground[3]
	return terrain == null or terrain.factor(seeker, point, point) > 0.0
