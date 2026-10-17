class_name FormationCommand
extends RefCounted
## A command (spec 30 round 3, the user's model): the orders a formation was set up with -
## the route it takes as its guide, the end it calls home, and the wave's standing options
## (hurrying, tending its own wounded, merging, pursuing). A unit carries the command it
## follows and the one it started under; a squad is a group of one command's units
## standing together, and every group of a command follows its orders. Units taken in by
## another group follow that group's command from then on (enlist). Pure.

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

## The route the command follows as its guide (Decision 75); null is the straight lane.
var route: FormationRoute = null
## Tiles along the route its home end lies at.
var home_distance := 0.0
## Ordered to hurry (Decision 125): its groups run while they move, spending stamina.
var hurry := false
## What it does with its own downed (Decision 126): "" leaves them, "recover" carries each
## home, "carry" bears them along.
var tends := ""
## Whether its groups merge into a friendly group they catch up with on the march.
var merges := false
## Whether its groups may pursue a retreating enemy (Decision 109).
var pursues := true
## Whether its groups walk out to take downed foes after a fight (FormationTaking), or
## leave them.
var takes := true
## Ways round its groups have found (RoutePatch, FormationPathing): every group follows the
## route with them.
var patches := []


func _init(home: float = 0.0, guide: FormationRoute = null) -> void:
	home_distance = home
	route = guide


## Takes `unit` into the group `squad` (a SkirmishSquad): it follows the group's command
## from now on, and keeps the one it started under.
static func enlist(squad, unit) -> void:
	unit.squad_id = squad.id
	unit.command = squad.command
	if unit.origin == null:
		unit.origin = squad.command
	squad.units.append(unit)
	UnitState.regather(squad)
