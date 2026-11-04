class_name ScrumRegroup
extends RefCounted
## Regrouping after a fight (Decisions 88 and 92, spec 27 rounds 5 and 6): units of squads
## out of the fight walk back to their places - in the squad's stance, if it holds one - at
## its re-form pace (FormationDiscipline), and rejoin its frame once in place and facing its
## way; one held off its place trades places or takes it (`trade.rs`, Decision 110). A
## drilled retreat's units back away facing the foe they touch (a fighting withdrawal,
## Decision 95). Every squad's units face the foes they touched as the phase began, so no
## squad listed earlier moves first and changes what a later one sees (Decision 97).
## Then the chasers (ScrumPursuit) chase; withdrawing units are left to FormationWithdraw.
## All of it is the crate's, over the scrum's roster (`scrum::regroup::step`, spec 39 step
## 8b5-xiv): each squad's say - whether it is out of the fight, its re-form pace, whether it
## is a drilled retreat or withdrawing, its chasers, and on terrain where its units walk
## back to (the pour's targets in its frame) - read where the crate holds it.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## Units of squads out of the fight walk to their places (in the stance, if any) at their
## re-form pace, paying the ground they cross (one whose place is barred makes for the
## nearest it can walk to it); back in place they rejoin the squad's frame unless it holds a
## stance. Then each chaser walks at the march pace after the nearest unit of the enemy it
## chases, or straggles back to its place once its chase is over (ScrumPursuit), on `field`
## (NativeKernels), over the scrum's `roster` (CrateRoster); `ctx` the scrum's (seed,
## seconds, pace, cells_per_second, terrain).
static func step(field: Object, roster: Object, ctx: Dictionary) -> void:
	var terrain: Object = ctx["terrain"]
	var ground: Object = terrain.core if terrain != null else null
	field.scrum_regroup(roster, ground, rules(ctx), ctx["seed"])


## The tuning the regroup and the chasers read, from the scrum's `ctx` (BodyField's
## `scrum_regroup`).
static func rules(ctx: Dictionary) -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			ctx["pace"],
			ctx["seconds"],
			tuning.reach_contact,
			tuning.reach_front_arc_degrees,
			tuning.bodies_steer_look,
			tuning.bodies_steer_clear,
			tuning.scrum_trade_seconds,
			ctx["cells_per_second"],
			tuning.stamina_give_up,
			tuning.discipline_per_leadership,
			tuning.discipline_meets_threats,
			tuning.discipline_march_pace_at,
			tuning.discipline_slowest,
			tuning.discipline_fastest,
			tuning.wounds_reach,
		]
	)
