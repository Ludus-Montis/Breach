class_name FormationManoeuvre
extends RefCounted
## A formation's current manoeuvre (Decisions 94 and 99, spec 27 rounds 7 and 10): it
## always has one, and the highest-priority one that applies wins - tactical decisions, its
## own, before the player's order:
## - **COMBAT (0):** locked in melee, front or flank, or skirmishing with an enemy in range.
##   Its units seek contact (Decision 88), even mid-re-form.
## - **WITHDRAW (1):** combat's equal (Decision 99): dealing with the enemy by leaving it.
##   Ordered to retreat from a fight, its units flee until it is safe (FormationWithdraw).
## - **ROUTE (1):** its units return to its route. Its frame never leaves the route, so its
##   units walking back from wherever a fight took them is this and RE_FORM in one.
## - **RE_FORM (2):** its units take up their places at its discipline's pace (Decision 92):
##   after a fight, a turn, or narrowing at a gap, or facing a threat closing in (a line
##   held only while that enemy keeps closing in, or, under a hold order, is still near).
## - **ORDER (3):** the player's order - march, hold, retreat, or wait (hold-until).
## A formation marches only when its manoeuvre is ORDER. Squads keep `manoeuvre`. Pure.

enum Kind { COMBAT, WITHDRAW, ROUTE, RE_FORM, ORDER }

const NAMES := ["combat", "withdraw", "route", "re-form", "order"]


## Settles each squad's manoeuvre this tick, in the crate (`manoeuvre.rs`, spec 39 step
## 4b; `scrum::pass::manoeuvre`, 8b5-xi), on the battle's `field` (null: no squads) over its
## squads' `roster` (CrateRoster). A gap's narrowing becomes a re-form: its units walk to
## their new places rather than the squad waiting a fixed time.
static func step(field: Object, roster: Object) -> void:
	if field != null:
		field.scrum_manoeuvre(roster)
