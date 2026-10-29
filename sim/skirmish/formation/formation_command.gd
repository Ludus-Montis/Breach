class_name FormationCommand
extends RefCounted
## A command (spec 30 round 3, the user's model): the orders a formation was set up with -
## the route it takes as its guide, the end it calls home, and the wave's standing options
## (hurrying, tending its own wounded, merging, pursuing). A unit carries the command it
## follows and the one it started under; a squad is a group of one command's units
## standing together, and every group of a command follows its orders. Units taken in by
## another group follow that group's command from then on (enlist). The crate holds it.

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

## Its flags in the crate (`commands.rs`).
const HURRY := 1
const MERGES := 2
const PURSUES := 4
const TAKES := 8

## The native BodyField holding it, and its slot there (spec 39 step 8b4-iii): the crate
## holds the command, this is a view of it.
var field: Object
var slot := -1
## The route the command follows as its guide (Decision 75); null is the straight lane.
var route: FormationRoute = null:
	set(value):
		route = value
		var points := PackedVector2Array() if value == null else value.points()
		var half_width := 0.0 if value == null else value.corridor_half_width
		field.command_put_route(slot, value != null, points, half_width)
## Tiles along the route its home end lies at.
var home_distance: float:
	get:
		return field.command_home(slot)
	set(value):
		field.put_command_home(slot, value)
## Ordered to hurry (Decision 125): its groups run while they move, spending stamina.
var hurry: bool:
	get:
		return field.command_gives(slot, HURRY)
	set(value):
		field.command_give(slot, HURRY, value)
## What it does with its own downed (Decision 126): "" leaves them, "recover" carries each
## home, "carry" bears them along.
var tends: String:
	get:
		return field.command_tends(slot)
	set(value):
		field.put_command_tends(slot, value)
## Whether its groups merge into a friendly group they catch up with on the march.
var merges: bool:
	get:
		return field.command_gives(slot, MERGES)
	set(value):
		field.command_give(slot, MERGES, value)
## Whether its groups may pursue a retreating enemy (Decision 109).
var pursues: bool:
	get:
		return field.command_gives(slot, PURSUES)
	set(value):
		field.command_give(slot, PURSUES, value)
## Whether its groups walk out to take downed foes after a fight (FormationTaking), or
## leave them.
var takes: bool:
	get:
		return field.command_gives(slot, TAKES)
	set(value):
		field.command_give(slot, TAKES, value)


## A command on `native_field` calling `home` its home, following `guide`.
func _init(native_field: Object, home: float = 0.0, guide: FormationRoute = null) -> void:
	field = native_field
	slot = field.command_open(home)
	route = guide


## Ways round its groups have found (RoutePatch, FormationPathing), which the crate keeps:
## every group follows the route with them.
func patch_count() -> int:
	return field.command_patch_count(slot)


## Keeps the way round `detour` (its waypoints, leaving and rejoining its route) as a patch.
func learn(detour: PackedVector2Array) -> void:
	field.command_learn(slot, detour)


## Takes `unit` into the group `squad` (a SkirmishSquad): it follows the group's command
## from now on, and keeps the one it started under.
static func enlist(squad, unit) -> void:
	unit.squad_id = squad.id
	unit.command = squad.command
	if unit.origin == null:
		unit.origin = squad.command
	squad.units.append(unit)
	UnitState.regather(squad)
