class_name SquadTurn
extends RefCounted
## How a squad turns as a block (Decision 74, specs/27-formations-in-2d.md):
## - a **wheel** (a quarter turn) pivots on the front centre and takes as long as the outer
##   end needs to march its quarter arc at the squad's pace
## - an **about-face** turns in place after a short pause; the back rank becomes the
##   front, and the re-form shuffle (Decision 46) moves front-preferring units forward
## Pure over the squad it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

const ABOUT_FACE_SECONDS := 1.0


## Ticks for a quarter turn of a squad `width` columns wide moving `cells_per_second`.
static func wheel_ticks(width: int, cells_per_second: float, tick_seconds: float) -> int:
	if cells_per_second <= 0.0:
		return 0
	var arc := PI / 2.0 * width / 2.0
	return maxi(1, ceili(arc / cells_per_second / tick_seconds - 0.000001))


static func about_face_ticks(tick_seconds: float) -> int:
	return maxi(1, roundi(ABOUT_FACE_SECONDS / tick_seconds))


## Turns the squad's places half round: ranks count from the old back, columns from the old
## right, and the centre shift flips with them. Moves under way are cancelled and the squad
## re-forms. Returns the squad's depth in ranks, how far its front moves back.
static func reverse_ranks(squad: SkirmishSquad) -> int:
	var depth := 0
	# store: scattered (few: turning about, once as a squad turns)
	for unit in squad.living():
		depth = maxi(depth, UnitPlace.rank(squad, unit) + unit.footprint_depth)
	# store: scattered (few: turning about, once as a squad turns)
	for unit in squad.living():
		var at := UnitPlace.of(squad, unit)
		var rank := depth - at.x - unit.footprint_depth
		UnitPlace.put(squad, unit, Vector2i(rank, squad.width - at.y - unit.footprint_width))
	squad.centre_shift = -squad.centre_shift
	squad.swaps.clear()
	squad.reforming = true
	return depth
