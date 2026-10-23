class_name ScrumSeek
extends RefCounted
## Seeking in the scrum (Decision 88, spec 30 round 1): each tick a fighting squad's free
## front-band units make for slots beside their foes' bodies, in contest order
## (ScrumContest) - first those keeping a slot still open, then the rest picking the
## nearest open one - so the earliest arrival takes a slot and the rest look further; with
## none open, it waits just behind the nearest (Decision 108), in a pursuit too (Decision
## 113). A unit touching a foe stands; one not free to seek keeps to its place. The rules
## run in the crate on the battle's native bodies (ScrumSeekField; the crate's `scrum.rs`
## and `seeking.rs`, spec 39 step 4a). Pure over the squads.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")

## Microseconds spent in plan() since it was last zeroed (the bench reads it; no outcome does).
static var clock_usec := 0


## Plans every fighting squad's units on the battle's native bodies, ctx["field"]
## (ScrumSeekField).
static func plan(ctx: Dictionary) -> void:
	var began := Time.get_ticks_usec()
	ScrumSeekField.plan(ctx)
	clock_usec += Time.get_ticks_usec() - began


## Ids of the squads it fights: on its front, its edges, or that fight it (the crate's
## `locks::foe_ids`, spec 39 step 4b).
static func foe_ids(squad: SkirmishSquad, squads: Array) -> Array:
	var codes := FormationLocks.factions([squad] + squads)
	var ids: PackedInt64Array = squad.store.field().squad_foe_ids(
		squad.slot, FormationLocks.slots(squads), codes.slice(1), codes[0]
	)
	return Array(ids)
