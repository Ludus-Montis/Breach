class_name ScrumNear
extends RefCounted
## The foes near a unit, found through a grid (BodyGrid) rather than by looking at every
## foe on the field (spec 30 round 3). An index is taken of [[unit, squad], ...] where they
## stand at that moment (ScrumReach); a search gives back the foes it could concern in the
## list's own order - every one that qualifies, and only some that don't - so whatever
## looks through them picks exactly as it would through the whole list (Decision 97).
## Pure; cells.

const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

## Cells added to every search's reach: far above float rounding, so none is missed.
const MARGIN := 0.01


## {"foes", "points": where each stands, "grid", "widest": the largest body's radius}.
static func index(foes: Array) -> Dictionary:
	var points := []
	var widest := 0.0
	var views := {}  # store -> where its units stand, read once (Decision 146)
	for entry in foes:
		var standing := UnitView.read_once(views, entry[1], ScrumReach.STANDING)
		points.append(ScrumReach.at_in(standing, entry[1], entry[0]))
		widest = maxf(widest, ScrumReach.radius(entry[0]))
	return {"foes": foes, "points": points, "grid": BodyGrid.build(points), "widest": widest}


## The foes whose bodies' edges may come within `reach` of `point`, in the list's order;
## of those of the squads its "only" names ({squad: true}), if it has one (FoeIndex).
static func around(near: Dictionary, point: Vector2, reach: float) -> Array:
	var foes: Array = near["foes"]
	var only = near.get("only")
	var out := []
	for found in BodyGrid.near(near["grid"], point, reach + near["widest"] + MARGIN):
		if only == null or only.has(foes[found][1]):
			out.append(foes[found])
	return out


## As ScrumSlots.gap_to: cells from `at` to the nearest foe body's edge, less `radius` (0
## if it touches one); INF with no foes. Looks ring by ring outwards, stopping once no
## nearer foe can lie further out. The last answer bounds the next (the nearest edge moves
## no more than the point asked about): rings nearer than it could be are passed over, and
## those beyond it never looked at.
static func gap_to(near: Dictionary, at: Vector2, radius: float) -> float:
	var foes: Array = near["foes"]
	var points: Array = near["points"]
	var grid: Dictionary = near["grid"]
	var only = near.get("only")  # as around()
	var centre := BodyGrid.cell_of(at)
	var span := BodyGrid.ring_span(grid, centre)
	var least := INF
	var limit := INF  # the least it can come to: the last answer and how far from it this is
	var last: Array = near.get("last", [at, INF])
	if last[1] < INF:
		var shift: float = last[0].distance_to(at)
		limit = last[1] + shift - radius + MARGIN
		var nearest: float = last[1] - shift - MARGIN  # no foe's centre is nearer than this
		span.x = maxi(span.x, ceili(nearest / (BodyGrid.CELL * sqrt(2.0))) - 1)
	for ring_number in range(span.x, span.y + 1):
		var bound := minf(least, limit)
		var widest: float = near["widest"]
		if BodyGrid.floor_of(ring_number) - widest - radius - MARGIN > bound:
			break
		for found in BodyGrid.ring(grid, centre, ring_number, at, bound + widest + radius):
			if only != null and not only.has(foes[found][1]):
				continue
			var edge: float = points[found].distance_to(at)
			least = minf(least, edge - ScrumReach.radius(foes[found][0]) - radius)
	near["last"] = [at, least + radius]
	return maxf(least, 0.0)


## The index (in the list) of the foe nearest `at` - by its distance snapped to 0.000001,
## then its unit's draw, then the list's order, as a search of the whole list in order
## would pick (Decision 97) - among those `accepts` (given the index) takes; -1 if none.
## Looks ring by ring outwards; a unit's draw is only worked out on a tie.
static func nearest(near: Dictionary, at: Vector2, fight_seed: int, accepts := Callable()) -> int:
	var grid: Dictionary = near["grid"]
	var centre := BodyGrid.cell_of(at)
	var span := BodyGrid.ring_span(grid, centre)
	var best := []  # [snapped distance, draw or null, unit, index, distance]
	for ring_number in range(span.x, span.y + 1):
		var within: float = INF if best.is_empty() else best[4] + MARGIN
		if BodyGrid.floor_of(ring_number) > within:
			break
		for found in BodyGrid.ring(grid, centre, ring_number, at, within):
			var apart: float = near["points"][found].distance_to(at)
			var unit: SkirmishUnit = near["foes"][found][0]
			var gap := snappedf(apart, 0.000001)
			if beats(best, gap, unit, fight_seed, found):
				if not accepts.is_valid() or accepts.call(found):
					best = [gap, null, unit, found, apart]
	return -1 if best.is_empty() else best[3]


## True if a unit `gap` away (snapped) comes before `best` ([snapped gap, draw or null,
## unit, index ...], [] for none): nearer, or as near with a lower draw, or the same draw
## and earlier in the list (`found`; -1 when searched in the list's order). Works out the
## draws only on a tie, keeping best's.
static func beats(
	best: Array, gap: float, unit: SkirmishUnit, fight_seed: int, found := -1
) -> bool:
	if best.is_empty() or gap < best[0]:
		return true
	if gap > best[0]:
		return false
	if best[1] == null:
		best[1] = ScrumContest.draw(best[2], fight_seed)
	var draw := ScrumContest.draw(unit, fight_seed)
	if draw != best[1]:
		return draw < best[1]
	return found >= 0 and found < best[3]
