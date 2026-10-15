class_name SkirmishSquad
extends RefCounted
## A wave in the field (specs/22-formation-feel-test.md, Decision 40): its units keep
## their formation places (rank, column) behind the squad's front distance, it moves as a
## block at its slowest unit's speed, and takes orders as a whole. Only the foremost unit
## of each column fights; when one falls the ranks behind step up (compact()).

enum State { MOVING, HOLDING, FIGHTING, ARRIVED, DESTROYED, ROUTING }

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")

## Tiles between one rank and the next: one cell (Decisions 48 and 68).
const RANK_DEPTH := 1.0 / MapLayoutDef.CELLS_PER_TILE

var id: int
## The battle's unit store (spec 35): its units' per-tick state, by their slots.
var store: UnitStore
var faction_id: String
## Which way the squad's front points along its route: +1 up it, -1 down it. An
## about-face flips it.
var direction: int
var home_distance: float
## Tiles along the route; setting it moves `position` with it.
var front_distance: float:
	set(value):
		front_distance = value
		_place()
## The route the squad follows (Decision 75); null is a straight lane along x.
var route: FormationRoute = null:
	set(value):
		route = value
		_place()
## Which way the squad's frame faces (Decision 105): a heading in degrees, clockwise from
## north (90 east), as a unit's bearing; its places are laid out in that turned frame.
var heading := 90.0
## The facing (SquadFrame) nearest its heading, for what still reckons in four ways (an
## exact diagonal rounds clockwise); setting it turns the frame to that way.
var facing: int:
	get:
		return posmod(roundi(heading / 90.0), 4)
	set(value):
		heading = posmod(value, 4) * 90.0
## The centre of its front edge in cells.
var position := Vector2.ZERO
var width: int
var order: SkirmishUnit.Order = SkirmishUnit.Order.ADVANCE
var state: State = State.MOVING
## The squad this one is fighting with its front; 0 = none.
var engaged_with: int = 0
## Hostile squads fighting it on its other edges (Decision 78): SquadEdges edge ->
## {"foe": squad id, "since": tick the contact began}.
var flank_contacts := {}
## The scrum (FormationScrum, Decision 88): its loose units (unit id -> the unit, in the
## order each went loose; each one's entry - where it stands loose, its next step, its goal
## and the rest - is the store's columns, and its LOOSE bit says it has one: SquadEntries,
## EntryKeys), the line it re-faced to {"anchor", "heading"},
## the tick its fight began (-1: none) and ticks with nobody able to strike.
var loose := {}
var stance := {}
var fight_since := -1
## Whether it has fought since it set out: groups of different commands form up only
## after both have (FormationGroups).
var fought := false
## Its units out taking the downed (FormationTaking): unit id -> the body; the tick they
## set out (-1: none), and the tick it gave up on them (-1: none) - it then marches on.
var taking := {}
var taking_since := -1
var took_until := -1
## Group pathfinding (FormationPathing): how many of its command's route patches its route
## carries, and where along it (cells) it last looked ahead.
var patched_count := 0
var looked_at := -INF
## Pursuit (ScrumPursuit, Decision 109): whether it may pursue a retreating enemy (false:
## ordered not to), and its units out chasing one on their own (unit id -> {"unit", "foe",
## "from", "leash"}).
var pursues: bool:
	get:
		return command.pursues
	set(value):
		command.pursues = value
var chasers := {}
## A pursuit under way (FormationPursuit): the enemy, the post it left, and how it held it.
var pursuit := {}
var stall_ticks := 0
## What it is doing (FormationManoeuvre.Kind, Decisions 94 and 99): 0 combat, 1
## withdrawing, 2 its route, 3 re-forming, 4 the player's order; it marches only on 4.
var manoeuvre := 4
## A withdrawal under way (FormationWithdraw): ticks it has felt safe; empty when none.
var withdraw := {}
## A hold-until order (FormationStaging, Decision 87); empty when it has none.
var staging := {}
## Its will to fight, 0 to 100 (FormationMorale, Decision 82); -1 until first read.
var morale := -1
## Routing (FormationRout): its fleeing units (unit id -> the unit; how far along its route
## each is, its place off the route and the rest are the store's columns, and its FLEEING
## bit says it has an entry: SquadEntries, EntryKeys), and ticks with no enemy near.
var fleeing := {}
var rally_ticks := 0
## Its command's orders (FormationCommand): hurrying and tending its own downed.
var hurry: bool:
	get:
		return command.hurry
	set(value):
		command.hurry = value
var tends: String:
	get:
		return command.tends
	set(value):
		command.tends = value
## Halted by ground it can't cross (FormationMarch.pace); reported once.
var blocked := false
## Narrowed through a gap (FormationNarrowing): its painted places (unit id -> [rank,
## column]), width and shift, and ticks left re-forming.
var painted := {}
var painted_width := 0
var painted_shift := 0.0
var narrow_ticks := 0
var wait_ticks: int = 0
## Re-forming after a reinforcement, and the swaps under way (Decision 46, FormationShuffle):
## [[mover, passed units, ticks left, ticks in all], ...].
var reforming := false
var swaps := []
var units: Array[SkirmishUnit] = []
## The lane's combat width: how wide joined units may spread the line in a fight (Decision
## 51); 0 keeps them within the squad's own columns.
var combat_width := 0
## Units that joined as reinforcements: only they spread beyond the painted columns.
var joined: Array[SkirmishUnit] = []
## Whether this wave merges into a friendly squad it catches up with on the march.
var merges: bool:
	get:
		return command.merges
	set(value):
		command.merges = value
## The command this group of units follows (spec 30 round 3).
var command: FormationCommand
## How far the squad's columns sit off the lane's centre, so widening on one side moves no
## one on screen.
var centre_shift := 0.0


func _init(
	squad_id: int,
	faction: String,
	travel_direction: int,
	home: float,
	formation_width: int,
	members: Array[SkirmishUnit] = [],
	orders: FormationCommand = null,
	unit_store: UnitStore = null
) -> void:
	# alone: its own, on the engine in use (a native one on its own field)
	store = (
		unit_store if unit_store != null else NativeKernels.unit_store(NativeKernels.body_field())
	)
	command = orders if orders != null else FormationCommand.new(home)
	id = squad_id
	faction_id = faction
	direction = travel_direction
	facing = SquadFrame.EAST if direction > 0 else SquadFrame.WEST
	home_distance = home
	front_distance = home
	width = maxi(formation_width, 1)
	for unit in members:
		unit.slot = store.enrol(unit.id)
		FormationCommand.enlist(self, unit)


func living() -> Array[SkirmishUnit]:
	return units.filter(func(u): return u.is_alive())


func is_destroyed() -> bool:
	return living().is_empty()


func speed() -> float:
	var slowest := INF
	for unit in living():
		slowest = minf(slowest, unit.speed)
	return 0.0 if slowest == INF else slowest


## Where a unit stands on the route: its rank's depth behind the squad's front.
func unit_distance(unit: SkirmishUnit) -> float:
	return front_distance - direction * UnitPlace.rank(self, unit) * RANK_DEPTH


## The unit's columns as a lateral span along `axis` (its squad's lateral axis if none):
## turned from its (rank, column), never mirrored (Decision 74). On a straight lane both
## sides share the lane's lateral axis, centred on it (less any centre_shift).
func lateral_span(unit: SkirmishUnit, axis := Vector2.ZERO) -> Vector2:
	var along := SquadFrame.lateral_axis(heading) if axis == Vector2.ZERO else axis
	var at := UnitPlace.of(self, unit)
	var points := SquadFrame.corners(position, heading, width, centre_shift, unit, at)
	return SquadFrame.extent(points, along)


## The front rank, left to right: only it fights in melee (Decision 47). A column whose
## front cell is empty has no fighter, so the enemy facing it wraps onto the flank.
func fighters() -> Array[SkirmishUnit]:
	var places := {}  # unit -> its place, read once
	for unit in living():
		places[unit] = UnitPlace.of(self, unit)
	var out: Array[SkirmishUnit] = living().filter(func(u): return places[u].x == 0)
	out.sort_custom(func(a, b): return places[a].y < places[b].y)
	return out


## Step-up: living units move forward a rank while their whole footprint would be clear
## (SquadCompact). Returns the units that stepped up.
func compact() -> Array[SkirmishUnit]:
	return SquadCompact.compact(self)


func _place() -> void:
	var cells := front_distance * MapLayoutDef.CELLS_PER_TILE
	position = route.point_at(cells) if route != null else Vector2(cells, 0.0)
