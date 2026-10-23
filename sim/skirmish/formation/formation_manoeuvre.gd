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

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")

const NAMES := ["combat", "withdraw", "route", "re-form", "order"]


## Settles each squad's manoeuvre this tick, in the crate (`manoeuvre.rs`, spec 39 step
## 4b). A gap's narrowing becomes a re-form: its units walk to their new places rather than
## the squad waiting a fixed time. This hands the crate what it doesn't hold: whether each
## squad is withdrawing and skirmishing (asked only where no higher priority applies), and
## the living units of one that narrowed.
static func step(squads: Array) -> void:
	if squads.is_empty():
		return
	var memo := SquadMemo.new()
	var cases := PackedInt64Array()
	for squad in squads:
		var withdrawing: bool = not squad.withdraw.is_empty()
		var fighting: bool = (
			squad.state == SkirmishSquad.State.FIGHTING
			or squad.engaged_with != 0
			or not squad.flank_contacts.is_empty()
		)
		var skirmishing := (
			not withdrawing and not fighting and FormationContact.skirmishing(squad, squads, memo)
		)
		var living: Array = squad.living() if squad.narrow_ticks > 0 else []
		cases.append_array([squad.slot, int(withdrawing), int(skirmishing), living.size()])
		for unit in living:
			cases.append_array([unit.id, unit.slot])
	squads[0].store.field().manoeuvre_step(cases)
