class_name FormationStaging
extends RefCounted
## Hold-until orders (Decision 87, spec 27 round 2). A staged wave marches to its staging
## point and holds there until its trigger, judged only on what it detects:
## - SEES_PARTNER: it detects its partner squad
## - SEES_FIGHT: it detects its partner (or, with no partner named, any friendly squad on
##   another route)
##   fighting
## or until its fallback runs out, when it goes on ("go") or turns back ("back"). Without a
## leader it goes as soon as the trigger fires. Led by a **coordinated** leader (Decision
## 81), a wave waiting to see its partner times its own departure: from the partner's
## distance to the meeting point and its pace, as seen, against its own predicted march,
## it goes when it would arrive no earlier than the partner - with no partner named, than
## every friend it sees firing the trigger, so none is singled out by the order squads are
## listed in (Decision 97). A squad's `staging`: {"at":
## cells along its route, "trigger", "partner": squad id or 0, "fallback": ticks, "then":
## "go" or "back", optionally "meet": the meeting point and "meet_cells": how far along its
## own route that is}. Pure. Whether the trigger has fired and it is time to go are the
## crate's (`staging.rs`, spec 39 step 5b), and holding and the fallback too (step 8a2-ii);
## the order and the events stay here.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")

const SEES_PARTNER := "sees_partner"
const SEES_FIGHT := "sees_fight"

## The crate's answers (staging::SIGNALLED, ON_ITS_WAY, HOLDS and GIVES_UP, plus STAGED).
const SIGNALLED := 0
const HOLDS := 2
const GIVES_UP := 3
const STAGED := 4


## True if the squad holds this tick: it is at its staging point and its trigger hasn't
## fired. Emits "staged" when it first holds, then "signalled" or "gave_up".
## `pace` = [cells a second at speed 1, seconds a tick].
## `terrain`, if given, can block sight. `roster`: the tick's (CrateRoster) of `squads`, made
## here if none is given.
static func holds(
	squad: SkirmishSquad,
	squads: Array,
	tick: int,
	events: Array,
	pace: Array = [8.0, 0.1],
	terrain: FormationTerrain = null,
	roster: Object = null
) -> bool:
	if squad.staging.is_empty() or squad.order != SkirmishUnit.Order.ADVANCE:
		return false
	var held := _hold(squad, squads, pace, terrain, roster)
	if held == SIGNALLED:
		squad.staging = {}
		events.append(FormationEvents.squad_event("signalled", tick, squad))
		return false
	if held & STAGED:
		events.append(FormationEvents.squad_event("staged", tick, squad))
	var what := held & ~STAGED
	if what == HOLDS:
		squad.staging["fallback"] -= 1
		return true
	if what != GIVES_UP:
		return false  # still on its way
	var then: String = squad.staging.get("then", "go")
	squad.staging = {}
	events.append(FormationEvents.squad_event("gave_up", tick, squad, {"then": then}))
	if then == "back":
		squad.order = SkirmishUnit.Order.RETREAT
	return false


## The crate's answer for the staged squad (staging::hold): whether its trigger has fired
## and it is time to go (a friend it detects fires it and, led by a coordinated leader with
## a meeting point, it would arrive no earlier than any of them), else whether it is at its
## staging point, holding while its fallback lasts.
static func _hold(
	squad: SkirmishSquad, squads: Array, pace: Array, terrain: FormationTerrain, roster: Object
) -> int:
	if roster == null:
		roster = CrateRoster.of(squads)
	var routes := {}  # route -> its code
	var codes := PackedInt32Array()
	for other in squads:
		codes.append(routes.get_or_add(other.route, routes.size()))
	var order := PackedInt64Array(
		[int(squad.staging["trigger"] != SEES_PARTNER), squad.staging.get("partner", 0)]
	)
	var meet := PackedFloat64Array()
	if squad.staging.has("meet") and _coordinated(squad):
		var at: Vector2 = squad.staging["meet"]
		meet = PackedFloat64Array(
			[at.x, at.y, squad.staging["meet_cells"], FormationWalk.slack_of(squad)]
		)
	return squad.store.field().staging_hold(
		roster,
		squads.find(squad),
		order,
		codes,
		meet,
		squad.route.points() if squad.route != null else PackedVector2Array(),
		PackedFloat64Array(pace),
		UnitMover.ground(terrain),
		float(squad.staging["at"]),
		int(squad.staging["fallback"])
	)


static func _coordinated(squad: SkirmishSquad) -> bool:
	return squad.living().any(func(u): return u.tactics.has("coordinated"))
