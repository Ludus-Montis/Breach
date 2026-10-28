class_name SquadTurn
extends RefCounted
## How a squad turns as a block (Decision 74, specs/27-formations-in-2d.md):
## - a **wheel** (a quarter turn) pivots on the front centre and takes as long as the outer
##   end needs to march its quarter arc at the squad's pace
## - an **about-face** turns in place after a short pause; the back rank becomes the
##   front, and the re-form shuffle (Decision 46) moves front-preferring units forward
## Pure. Reversing the ranks is the crate's (`turn.rs`, ScrumTurn, spec 39 step 8a2-i).

const ABOUT_FACE_SECONDS := 1.0


## Ticks for a quarter turn of a squad `width` columns wide moving `cells_per_second`.
static func wheel_ticks(width: int, cells_per_second: float, tick_seconds: float) -> int:
	if cells_per_second <= 0.0:
		return 0
	var arc := PI / 2.0 * width / 2.0
	return maxi(1, ceili(arc / cells_per_second / tick_seconds - 0.000001))


static func about_face_ticks(tick_seconds: float) -> int:
	return maxi(1, roundi(ABOUT_FACE_SECONDS / tick_seconds))
