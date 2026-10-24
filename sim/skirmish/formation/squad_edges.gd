class_name SquadEdges
extends RefCounted
## A squad's four edges (Decision 78, spec 27 round 2): front, right, rear and left,
## relative to its heading (clockwise, as headings are). Which edge an attacker meets, which
## units stand on an edge (the outermost on that side, so when one falls the next inward
## is the edge: step-up inward is implicit), and how far an attacker's front is from a
## squad's near face. On a turned frame each edge is a quarter of the way round, its
## footprints turned with it. Pure; cells unless stated. The geometry is the crate's
## (`geometry.rs`, spec 39 step 4c), on a roster (CrateRoster) of the squad.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

const FRONT := 0
const RIGHT := 1
const REAR := 2
const LEFT := 3


## The cells the squad's living units cover. `listed`: [a roster (CrateRoster) holding it,
## its index there], made here if empty.
static func bounds(squad: SkirmishSquad, listed := []) -> Rect2:
	if listed.is_empty():
		listed = [CrateRoster.of([squad]), 0]
	return squad.store.field().squad_bounds(listed[0], listed[1])


## How far the squad's living footprints reach along `way`: (least, most). `listed` as for
## bounds.
static func reach(squad: SkirmishSquad, way: Vector2, listed := []) -> Vector2:
	if listed.is_empty():
		listed = [CrateRoster.of([squad]), 0]
	return squad.store.field().squad_reach(listed[0], listed[1], way)
