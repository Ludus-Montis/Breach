class_name StoredSquad
extends RefCounted
## A squad's state in the squad store (specs/39-simulation-in-rust.md step 1): each field
## below that a pass reads or writes is a column of its store's squad columns
## (SquadColumns, on the crate's `Squads` under Rust) at its slot, read and written through
## these properties, so a pass reads a squad as before. SkirmishSquad is one.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadColumns = preload("res://sim/skirmish/formation/squad_columns.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

var id: int
## Its slot in its store's squad columns, kept for the battle.
var slot := -1
## The battle's unit store (spec 35): its units' per-tick state, by their slots, and its own
## state in the squad columns. Set to another store, it carries its state there (a test that
## moves a battle's units), and its members are written there at its lists' next rebuild.
var store: UnitStore:
	set(value):
		if store == null or store == value:
			store = value
			return
		var row := []
		for column in SquadColumns.BLANKS.size():
			row.append(store.squads.value(slot, column))
		var records: SquadRecords = store.records
		var orders: OrderRecords = store.orders
		var was := slot
		store = value
		slot = value.squads.open(id)
		for column in row.size():
			value.squads.put(slot, column, row[column])
		value.records.carry(records, was, slot)
		value.orders.carry(orders, was, slot)
		UnitState.regather(self)
## Which way the squad's front points along its route: +1 up it, -1 down it. An
## about-face flips it.
var direction: int:
	get:
		return store.squads.value(slot, SquadColumns.DIRECTION)
	set(value):
		store.squads.put(slot, SquadColumns.DIRECTION, value)
## Tiles along its route where it set out.
var home_distance: float:
	get:
		return store.squads.value(slot, SquadColumns.HOME_DISTANCE)
	set(value):
		store.squads.put(slot, SquadColumns.HOME_DISTANCE, value)
## Tiles along the route; setting it moves `position` with it.
var front_distance: float:
	get:
		return store.squads.value(slot, SquadColumns.FRONT_DISTANCE)
	set(value):
		store.squads.put(slot, SquadColumns.FRONT_DISTANCE, value)
		_place()
## Which way the squad's frame faces (Decision 105): a heading in degrees, clockwise from
## north (90 east), as a unit's bearing; its places are laid out in that turned frame.
var heading: float:
	get:
		return store.squads.value(slot, SquadColumns.HEADING)
	set(value):
		store.squads.put(slot, SquadColumns.HEADING, value)
## The centre of its front edge in cells.
var position: Vector2:
	get:
		return store.squads.value(slot, SquadColumns.POSITION)
	set(value):
		store.squads.put(slot, SquadColumns.POSITION, value)
## Its columns.
var width: int:
	get:
		return store.squads.value(slot, SquadColumns.WIDTH)
	set(value):
		store.squads.put(slot, SquadColumns.WIDTH, value)
## The order it is under.
var order: SkirmishUnit.Order:
	get:
		return store.squads.value(slot, SquadColumns.ORDER)
	set(value):
		store.squads.put(slot, SquadColumns.ORDER, value)
## What it is doing as a whole (SkirmishSquad.State).
var state: int:
	get:
		return store.squads.value(slot, SquadColumns.STATE)
	set(value):
		store.squads.put(slot, SquadColumns.STATE, value)
## The squad this one is fighting with its front; 0 = none.
var engaged_with: int:
	get:
		return store.squads.value(slot, SquadColumns.ENGAGED_WITH)
	set(value):
		store.squads.put(slot, SquadColumns.ENGAGED_WITH, value)
## The tick its scrum's fight began (-1: none) (FormationScrum).
var fight_since: int:
	get:
		return store.squads.value(slot, SquadColumns.FIGHT_SINCE)
	set(value):
		store.squads.put(slot, SquadColumns.FIGHT_SINCE, value)
## Whether it has fought since it set out: groups of different commands form up only
## after both have (FormationGroups).
var fought: bool:
	get:
		return store.squads.value(slot, SquadColumns.FOUGHT) != 0
	set(value):
		store.squads.put(slot, SquadColumns.FOUGHT, int(value))
## The tick its units set out taking the downed (-1: none), and the tick it gave up on
## them (-1: none) - it then marches on (FormationTaking).
var taking_since: int:
	get:
		return store.squads.value(slot, SquadColumns.TAKING_SINCE)
	set(value):
		store.squads.put(slot, SquadColumns.TAKING_SINCE, value)
var took_until: int:
	get:
		return store.squads.value(slot, SquadColumns.TOOK_UNTIL)
	set(value):
		store.squads.put(slot, SquadColumns.TOOK_UNTIL, value)
## Group pathfinding (FormationPathing): how many of its command's route patches its route
## carries, and where along it (cells) it last looked ahead.
var patched_count: int:
	get:
		return store.squads.value(slot, SquadColumns.PATCHED_COUNT)
	set(value):
		store.squads.put(slot, SquadColumns.PATCHED_COUNT, value)
var looked_at: float:
	get:
		return store.squads.value(slot, SquadColumns.LOOKED_AT)
	set(value):
		store.squads.put(slot, SquadColumns.LOOKED_AT, value)
## Ticks of its pursuit with nobody able to strike (FormationPursuit).
var stall_ticks: int:
	get:
		return store.squads.value(slot, SquadColumns.STALL_TICKS)
	set(value):
		store.squads.put(slot, SquadColumns.STALL_TICKS, value)
## What it is doing (FormationManoeuvre.Kind, Decisions 94 and 99): 0 combat, 1
## withdrawing, 2 its route, 3 re-forming, 4 the player's order; it marches only on 4.
var manoeuvre: int:
	get:
		return store.squads.value(slot, SquadColumns.MANOEUVRE)
	set(value):
		store.squads.put(slot, SquadColumns.MANOEUVRE, value)
## Its will to fight, 0 to 100 (FormationMorale, Decision 82); -1 until first read.
var morale: int:
	get:
		return store.squads.value(slot, SquadColumns.MORALE)
	set(value):
		store.squads.put(slot, SquadColumns.MORALE, value)
## Routing: ticks with no enemy near (FormationRout).
var rally_ticks: int:
	get:
		return store.squads.value(slot, SquadColumns.RALLY_TICKS)
	set(value):
		store.squads.put(slot, SquadColumns.RALLY_TICKS, value)
## Halted by ground it can't cross (FormationMarch.pace); reported once.
var blocked: bool:
	get:
		return store.squads.value(slot, SquadColumns.BLOCKED) != 0
	set(value):
		store.squads.put(slot, SquadColumns.BLOCKED, int(value))
## Narrowed through a gap (FormationNarrowing): its painted width and shift, and ticks
## left re-forming.
var painted_width: int:
	get:
		return store.squads.value(slot, SquadColumns.PAINTED_WIDTH)
	set(value):
		store.squads.put(slot, SquadColumns.PAINTED_WIDTH, value)
var painted_shift: float:
	get:
		return store.squads.value(slot, SquadColumns.PAINTED_SHIFT)
	set(value):
		store.squads.put(slot, SquadColumns.PAINTED_SHIFT, value)
var narrow_ticks: int:
	get:
		return store.squads.value(slot, SquadColumns.NARROW_TICKS)
	set(value):
		store.squads.put(slot, SquadColumns.NARROW_TICKS, value)
## Ticks it waits before it sets out.
var wait_ticks: int:
	get:
		return store.squads.value(slot, SquadColumns.WAIT_TICKS)
	set(value):
		store.squads.put(slot, SquadColumns.WAIT_TICKS, value)
## Re-forming after a reinforcement (Decision 46, FormationShuffle).
var reforming: bool:
	get:
		return store.squads.value(slot, SquadColumns.REFORMING) != 0
	set(value):
		store.squads.put(slot, SquadColumns.REFORMING, int(value))
## The lane's combat width: how wide joined units may spread the line in a fight (Decision
## 51); 0 keeps them within the squad's own columns.
var combat_width: int:
	get:
		return store.squads.value(slot, SquadColumns.COMBAT_WIDTH)
	set(value):
		store.squads.put(slot, SquadColumns.COMBAT_WIDTH, value)
## How far the squad's columns sit off the lane's centre, so widening on one side moves no
## one on screen.
var centre_shift: float:
	get:
		return store.squads.value(slot, SquadColumns.CENTRE_SHIFT)
	set(value):
		store.squads.put(slot, SquadColumns.CENTRE_SHIFT, value)

## Hostile squads fighting it on its other edges (Decision 78): SquadEdges edge ->
## {"foe": squad id, "since": tick the contact began}. A view of its records (SquadRecords):
## set one with `flank`, clear them with `unflank_all`.
var flank_contacts: Dictionary:
	get:
		return store.records.flanks(slot)
## The scrum (FormationScrum, Decision 88): its loose units (unit id -> the unit, in the
## order each went loose; each one's entry - where it stands loose, its next step, its goal
## and the rest - is the store's columns, and its LOOSE bit says it has one: SquadEntries,
## EntryKeys). A view of its records, given and taken away by SquadEntries.
var loose: Dictionary:
	get:
		return store.records.loose(slot)
## The line it re-faced to (ScrumStance), {"anchor", "heading", "foe"}; {} if none.
var stance: Dictionary:
	get:
		return store.records.stance(slot)
	set(value):
		store.records.put_stance(slot, value)
## Its units out chasing an enemy on their own (ScrumPursuit, Decision 109): unit id ->
## {"unit", "foe", "from", "leash"}, and "returning" once it turns back. A view of its
## records: set one with `chase`, turn it back with `chase_back`, end it with `unchase`.
var chasers: Dictionary:
	get:
		return store.records.chasers(slot)
## Routing (FormationRout): its fleeing units (unit id -> the unit; how far along its route
## each is, its place off the route and the rest are the store's columns, and its FLEEING
## bit says it has an entry: SquadEntries, EntryKeys). A view of its records.
var fleeing: Dictionary:
	get:
		return store.records.fleeing(slot)


## Sets the foe (a squad id) on its `edge` (SquadEdges), its contact begun at `since`.
func flank(edge: int, foe: int, since: int) -> void:
	store.field().put_squad_flank(slot, edge, foe, since)


## Takes every flank contact away.
func unflank_all() -> void:
	store.field().clear_squad_flanks(slot)


## Sends `unit` out chasing the squad `foe` (its id) from `from`, on a `leash` of cells.
func chase(unit, foe: int, from: Vector2, leash: float) -> void:
	store.field().put_squad_chaser(slot, unit.id, foe, from, leash)


## The unit out chasing gives up and makes its way back ("returning").
func chase_back(unit_id: int) -> void:
	store.field().return_squad_chaser(slot, unit_id)


## Calls the unit back from its chase (nothing if it isn't out).
func unchase(unit_id: int) -> void:
	store.field().drop_squad_chaser(slot, unit_id)


## Its frame read once a pass (Decision 144): [its position, heading, width, centre shift],
## for SquadFrame's place(), corners() and span() over many of its units.
func frame() -> Array:
	return [position, heading, width, centre_shift]


## Puts its front where its front distance says along its route.
func _place() -> void:
	position = _point_at(front_distance * MapLayoutDef.CELLS_PER_TILE)


## The point `cells` along its route: here a straight lane along x.
func _point_at(cells: float) -> Vector2:
	return Vector2(cells, 0.0)
