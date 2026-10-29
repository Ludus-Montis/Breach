class_name SkirmishSquad
extends StoredSquad
## A wave in the field (specs/22-formation-feel-test.md, Decision 40): its units keep
## their formation places (rank, column) behind the squad's front distance, it moves as a
## block at its slowest unit's speed, and takes orders as a whole. Only the foremost unit
## of each column fights; when one falls the ranks behind step up (compact()).

enum State { MOVING, HOLDING, FIGHTING, ARRIVED, DESTROYED, ROUTING }

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")

## Tiles between one rank and the next: one cell (Decisions 48 and 68).
const RANK_DEPTH := 1.0 / MapLayoutDef.CELLS_PER_TILE

var faction_id: String
## The route the squad follows (Decision 75); null is a straight lane along x. The crate
## holds it too (spec 39 step 8b4-i), and the bindings that follow it read it there.
var route: FormationRoute = null:
	set(value):
		route = value
		var points := PackedVector2Array() if value == null else value.points()
		var half_width := 0.0 if value == null else value.corridor_half_width
		store.field().squad_put_route(slot, value != null, points, half_width)
		_place()
## The facing (SquadFrame) nearest its heading, for what still reckons in four ways (an
## exact diagonal rounds clockwise); setting it turns the frame to that way.
var facing: int:
	get:
		return posmod(roundi(heading / 90.0), 4)
	set(value):
		heading = posmod(value, 4) * 90.0
## Its units out taking the downed (FormationTaking): taker's id -> its body's store slot,
## as the crate holds it (spec 39 step 8b4-vi), read and written whole; the tick they set out
## (-1: none), and the tick it gave up on them (-1: none) - it then marches on.
var taking: Dictionary:
	get:
		return store.field().squad_taking(slot)
	set(value):
		store.field().put_squad_taking(slot, value)
## Pursuit (ScrumPursuit, Decision 109): whether it may pursue a retreating enemy (false:
## ordered not to); its units out chasing one on their own are `chasers` (StoredSquad).
var pursues: bool:
	get:
		return command.pursues
	set(value):
		command.pursues = value
## A pursuit under way (FormationPursuit): the enemy, the post it left, and how it held it;
## empty when none. A view of its records (OrderRecords), set by the crate's orders and
## pursuit (spec 39 step 7c).
var pursuit: Dictionary:
	get:
		return store.orders.pursuit(slot)
## A withdrawal under way (FormationWithdraw): ticks it has felt safe; empty when none. A
## view of its records, set by the crate's orders and withdrawal.
var withdraw: Dictionary:
	get:
		return store.orders.withdraw(slot)
## A hold-until order (FormationStaging, Decision 87); empty when it has none. A view of
## the crate's (`staging::Stage`, spec 39 step 8b4-iv): read and written whole.
var staging: Dictionary:
	get:
		return store.field().squad_staging(slot)
	set(value):
		store.field().put_squad_staging(slot, value)
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
## The swaps of its re-forming under way (FormationShuffle), as the crate holds them (spec
## 39 step 8b4-v): each {"ticks", "total", "mover", "to", "from", "foot", "passed"}, units by
## id. Read and written whole; FormationShuffle.swapping() and clear() ask and cancel them.
var swaps: Array:
	get:
		return store.field().squad_swaps(slot)
	set(value):
		store.field().put_squad_swaps(slot, value)
var units: Array[SkirmishUnit] = []
## Its living and downed units, kept by UnitState (Decision 147).
var kept := UnitState.Kept.new()
## Units that joined as reinforcements: only they spread beyond the painted columns.
var joined: Array[SkirmishUnit] = []
## Whether this wave merges into a friendly squad it catches up with on the march.
var merges: bool:
	get:
		return command.merges
	set(value):
		command.merges = value
## The command this group of units follows (spec 30 round 3); the crate knows it too.
var command: FormationCommand:
	set(value):
		command = value
		store.field().squad_put_command(slot, value.slot)


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
	slot = store.squads.open(squad_id)
	command = orders if orders != null else FormationCommand.new(store.field(), home)
	id = squad_id
	faction_id = faction
	direction = travel_direction
	facing = SquadFrame.EAST if direction > 0 else SquadFrame.WEST
	home_distance = home
	front_distance = home
	width = maxi(formation_width, 1)
	for unit in members:
		store.admit(unit)
		UnitState.admitted(store, unit)
		FormationCommand.enlist(self, unit)


## Its standing units, in its order: kept by UnitState, so asking costs nothing. Read-only.
func living() -> Array[SkirmishUnit]:
	return UnitState.living(self)


func is_destroyed() -> bool:
	return living().is_empty()


## Its slowest living unit's pace this tick (the store's SPEED, read once).
func speed() -> float:
	var slowest := INF
	var paces: PackedFloat64Array = store.columns.column(UnitColumns.SPEED)
	for unit in living():
		slowest = minf(slowest, paces[unit.slot])
	return 0.0 if slowest == INF else slowest


## Where a unit of `rank` stands on the route: its rank's depth behind the squad's front.
func rank_distance(rank: int) -> float:
	return front_distance - direction * rank * RANK_DEPTH


## The unit's columns as a lateral span along `axis` (its squad's lateral axis if none):
## turned from its (rank, column), never mirrored (Decision 74). On a straight lane both
## sides share the lane's lateral axis, centred on it (less any centre_shift).
## `at`: the unit's place (rank, column), from a pass's view (UnitPlace.in_view) or, for an
## event's few, the store (UnitPlace.of).
func lateral_span(unit: SkirmishUnit, axis: Vector2, at: Vector2i) -> Vector2:
	return SquadFrame.span(frame(), unit, axis, at)


## The front rank, left to right: only it fights in melee (Decision 47). A column whose
## front cell is empty has no fighter, so the enemy facing it wraps onto the flank.
func fighters() -> Array[SkirmishUnit]:
	var places := UnitView.read(self, UnitView.PLACES)  # read once (Decision 146)
	var ranks: PackedInt32Array = places.rank
	var files: PackedInt32Array = places.column
	var out: Array[SkirmishUnit] = living().filter(func(u): return ranks[u.slot] == 0)
	out.sort_custom(func(a, b): return files[a.slot] < files[b.slot])
	return out


## Step-up: living units move forward a rank while their whole footprint would be clear
## (SquadCompact). Returns the units that stepped up.
func compact() -> Array[SkirmishUnit]:
	return SquadCompact.compact(self)


func _point_at(cells: float) -> Vector2:
	return route.point_at(cells) if route != null else super(cells)
