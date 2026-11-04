class_name ScrumPursuit
extends RefCounted
## Retreat and pursuit (Decisions 95 and 109, spec 27 round 8). When a formation is
## ordered to retreat out of a fight:
## - **The retreat:** its locks end at once and it withdraws (FormationWithdraw, Decision
##   99): its units flee from where they stand until it is safe. A ragged one also pays a
##   scaled rout: a morale shock in proportion to how far short of drilled it is.
## - **Its enemies:** any of their units still touching it strike it as it goes
##   (ScrumBlows). Each follows it as a whole, to the leash its discipline gives it
##   (FormationPursuit, Decision 107), unless ordered not to pursue. And each of their units
##   near it may break ranks to chase on its own - decided per unit, by its discipline and a
##   seeded roll - as far from where it broke away as its own discipline leashes it, while
##   it can see its quarry; then it goes back to its formation.
## Chasers all pick their quarry before any moves: the nearest unit, ties by its seeded
## draw (Decision 97).
## Squads keep `pursues` and `chasers` (unit id -> {"unit", "foe", "from", "leash"}). The
## chase is the crate's (`pursuit.rs`, spec 39 step 4c), run after the regroup over the
## scrum's roster, each squad's chasers and where they walk back to on the ground read there
## (ScrumRegroup, step 8b5-xiv). The retreat itself, and who is tempted to break ranks, are
## the crate's orders (FormationOrders, step 7c).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


## True if the squad's unit is away from its place on its own - chasing, or its squad
## withdrawing - so its squad leaves it out of regrouping.
static func away(squad: SkirmishSquad, unit_id: int) -> bool:
	return squad.chasers.has(unit_id) or not squad.withdraw.is_empty()
