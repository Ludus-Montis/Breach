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
## crate's (`staging.rs`, spec 39 step 5b), and holding and the fallback too (step 8a2-ii),
## and the order itself (step 8b4-iv: SkirmishSquad.staging is a view of it), and its
## events (the move's pass, step 8b5-viii), and the routes' codes (step 8b5-xxxvi: the same
## code, the same route; a wave on its own route is no partner for one with no partner named).

const SEES_PARTNER := "sees_partner"
const SEES_FIGHT := "sees_fight"
