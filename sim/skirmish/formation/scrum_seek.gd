class_name ScrumSeek
extends RefCounted
## Seeking in the scrum (Decision 88, spec 30 round 1): each tick a fighting squad's free
## front-band units make for slots beside their foes' bodies, in contest order
## (ScrumContest) - first those keeping a slot still open, then the rest picking the
## nearest open one - so the earliest arrival takes a slot and the rest look further; with
## none open, it waits just behind the nearest (Decision 108), in a pursuit too (Decision
## 113). A unit touching a foe stands; one not free to seek keeps to its place. Then the
## walk: each steps from the bodies as they stood before any stepped, one making for a slot
## towards its next point, one with no slot that touches no foe back towards its place,
## round the bodies in its way (`steer.rs`), over those lying underfoot (GroundBodies),
## paying the ground it crosses; units out chasing on their own are left to ScrumPursuit.
## Then the facing, after all have moved so a unit stepped up to is seen: each turns towards
## the nearest foe it touches, or else so as to arrive facing what it will do
## (`motion::look`). All of it is the crate's, over the scrum's roster
## (`scrum::seek::step`, spec 39 step 8b5-xiii): the field synced, each squad's say - its
## foes, whether it may seek, its frame, its chasers - read where the crate holds it.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

## Microseconds spent in step() since it was last zeroed (the bench reads it; no outcome does).
static var clock_usec := 0


## Seeks, walks and turns every fighting squad's units on `field` (NativeKernels), over the
## scrum's `roster` (CrateRoster); `ctx` the scrum's (tick, seed, seconds, pace, terrain).
## Each squad active this tick - one of whose units touches a foe or makes for a slot - goes
## into ctx["active"] by its id, for the stall.
static func step(field: Object, roster: Object, ctx: Dictionary) -> void:
	var began := Time.get_ticks_usec()
	var tuning := BattleTuning.current()
	var ints := PackedInt64Array([ctx["tick"], ctx["seed"], tuning.combat_contest_die])
	var floats := PackedFloat64Array(
		[
			tuning.reach_contact,
			tuning.scrum_leash,
			ctx["seconds"],
			ctx["pace"],
			tuning.scrum_crowding,
			tuning.bodies_steer_look,
			tuning.bodies_steer_clear,
			tuning.bodies_ground_drag,
			tuning.bodies_ground_block,
			DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001,
		]
	)
	var terrain: Object = ctx["terrain"]
	var ground: Object = terrain.core if terrain != null else null
	var active: PackedInt64Array = field.scrum_seek(roster, ground, ints, floats)
	for squad_id in active:
		ctx["active"][squad_id] = true
	clock_usec += Time.get_ticks_usec() - began
