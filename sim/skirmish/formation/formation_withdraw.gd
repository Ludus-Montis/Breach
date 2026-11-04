class_name FormationWithdraw
extends RefCounted
## A withdrawal (Decision 99, spec 27 round 10): a retreat is how a formation deals with an
## enemy, as fighting is, so it shares combat's priority (FormationManoeuvre). Ordered to
## retreat out of a fight, it leaves from where its units stand - no walking back to their
## places first:
## - **Disengaging, then flight:** each unit turns for home and heads there along its
##   route. Disengaging is a manoeuvre whose efficacy its formation's discipline sets, as
##   re-forming is (Decisions 92 and 101): until clear of the foes it touches, a unit turns
##   at its turn rate, and steps away at its pace, times the re-form pace - a disciplined
##   formation breaks off quickly and cleanly, a ragged one slowly, backing off and exposed
##   (flank blows) while it turns. Once clear it is in flight, at its full pace. Until it
##   has turned, a unit still facing a foe it touches strikes it (FormationMelee). The less
##   ordered the formation, the wider its units fan out from the route's line: up to
##   rout_fan_degrees (BattleTuning) either side, by a seeded angle per unit, for one with no
##   discipline. Ground it can't cross turns it back onto the route (a ford it fanned
##   away from), and holds it only if that is barred too.
## - **Safe:** with no enemy within rout_enemy_near of it, and none pursuing it,
##   for rout_rally_seconds - the test a rout rallies by - it re-forms on its route
##   where its units stand, facing home, and marches home (its order). One whose units are
##   all home, with nowhere further to go, re-forms there at once, facing out the way it
##   will hold, and fights as any other. Its units fan out no further than a rout's, by its
##   disorder.
## Squads keep `withdraw` ({"safe_ticks"}), a view of their records (OrderRecords). Its start
## is the crate's orders (FormationOrders); the rest is the crate's over the scrum's roster
## (`scrum::withdraw::step`, spec 39 step 8b5-xv): each withdrawing squad's disorder and
## re-form pace (from its units' sheets), its route and home and its units' flight angles
## (RoutFlight.fan) read where the crate holds them, the flight, who is safe or home, its
## record, its re-forming and its turn (ScrumTurn), and "turning", "turned", "regrouping"
## and "returned" in the crate's log.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")


## How far from orderly the squad is, 0 (drilled) to 1 (no discipline): how wide it fans.
static func disorder(squad: SkirmishSquad) -> float:
	if FormationDiscipline.meets_threats(squad):
		return 0.0
	return (
		1.0 - float(FormationDiscipline.of(squad)) / BattleTuning.current().discipline_meets_threats
	)


## One tick of withdrawals, in the crate on `field` (NativeKernels) over the scrum's `roster`
## (CrateRoster): units flee, and squads that have been safe long enough re-form; their
## events are appended to `events`. `ctx`: the scrum's (tick, pace, seconds, seed, terrain).
static func step(field: Object, roster: Object, ctx: Dictionary, events: Array) -> void:
	var terrain: Object = ctx["terrain"]
	var ground: Object = terrain.core if terrain != null else null
	field.scrum_withdraw(roster, ground, rules(ctx), ctx["tick"], ctx["seed"])
	events.append_array(CrateEvents.taken(field))


## The tuning the withdrawal reads, from the scrum's `ctx` (BodyField's `scrum_withdraw`).
static func rules(ctx: Dictionary) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			ctx["pace"],
			ctx["seconds"],
			tuning.reach_contact,
			tuning.rout_enemy_near,
			tuning.rout_fan_cells,
			tuning.rout_rally_seconds,
			tuning.rout_fan_degrees,
			tuning.discipline_per_leadership,
			tuning.discipline_meets_threats,
			tuning.discipline_march_pace_at,
			tuning.discipline_slowest,
			tuning.discipline_fastest,
		]
	)
