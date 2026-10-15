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
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const NAMES := ["combat", "withdraw", "route", "re-form", "order"]


## Settles each squad's manoeuvre this tick. A gap's narrowing becomes a re-form: its
## units walk to their new places rather than the squad waiting a fixed time.
static func step(squads: Array) -> void:
	var memo := SquadMemo.new()  # loosening a squad moves no frame or unit
	for squad in squads:
		if squad.narrow_ticks > 0:
			squad.narrow_ticks = 0
			_loosen(squad)
		squad.manoeuvre = current(squad, squads, memo)


## The highest-priority manoeuvre that applies to the squad now. `memo`: the phase's
## (SquadMemo).
static func current(squad: SkirmishSquad, squads: Array, memo: SquadMemo = null) -> Kind:
	if not squad.withdraw.is_empty():
		return Kind.WITHDRAW
	var fighting := (
		squad.state == SkirmishSquad.State.FIGHTING
		or squad.engaged_with != 0
		or not squad.flank_contacts.is_empty()
	)
	if fighting or FormationContact.skirmishing(squad, squads, memo):
		return Kind.COMBAT
	if not squad.loose.is_empty() or not squad.stance.is_empty():
		return Kind.RE_FORM
	return Kind.ORDER


## Its units walk from where they stand to their new places.
static func _loosen(squad: SkirmishSquad) -> void:
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	for unit in squad.living():
		if not squad.loose.has(unit.id):
			var stood := at[unit.slot]
			SquadEntries.loosen(squad, unit, stood)
