extends GdUnitTestSuite
## Units move through one mover, which pays the ground (Decision 139, UnitMover): a router
## fleeing through a wood goes at half its open-ground pace, as a march or a walk would;
## the mover's step on open ground is the unit's own walk, unchanged.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")

const SIZE := Vector2i(256, 32)
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _wood() -> FormationTerrain:
	var terrain := FormationTerrain.new(SIZE)
	terrain.paint(Rect2i(Vector2i.ZERO, SIZE), {"cost": 0.5})
	return terrain


func _unit() -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = 1
	unit.speed = 1.0
	return unit


## How far along its route a lone router gets in one step, on `terrain` (null: open).
func _flight(terrain) -> float:
	var squad := SkirmishSquad.new(
		1, "player", 1, 0.0, 1, [] as Array[SkirmishUnit], null, _places.store
	)
	squad.route = FormationRoute.new(PackedVector2Array([Vector2(0, 16), Vector2(256, 16)]))
	var unit: SkirmishUnit = _places.stand(_unit(), Vector2(100, 16))  # its slot in the store
	SquadEntries.flee(squad, unit, 100.0, Vector2.ZERO)
	RoutFlight.step(squad, unit, 0.0, [1.0, 7, terrain, Vector2(100, 16), {}])
	return 100.0 - squad.store.columns.value(unit.slot, UnitColumns.ALONG)


func test_a_router_in_a_wood_flees_at_half_pace() -> void:
	var open := _flight(null)
	var wood := _flight(_wood())
	assert_float(open).is_greater(0.1)
	assert_float(wood / open).is_equal_approx(0.5, 0.01)


func test_on_open_ground_the_mover_is_the_units_own_walk() -> void:
	var columns := _places.store.columns
	var walker: SkirmishUnit = _places.stand(_unit(), Vector2(10, 10))  # each facing east
	var mover := _unit()
	mover.id = 2  # a slot of its own
	_places.stand(mover, Vector2(10, 10))
	var walked := UnitMotion.walk(walker, columns, Vector2(10, 10), Vector2(20, 12), 0.7, 0.1)
	var moved := UnitMover.step(mover, columns, Vector2(10, 10), Vector2(20, 12), 0.7, 0.1, null)
	assert_vector(moved).is_equal(walked)


func test_a_step_into_ground_it_cannot_cross_is_refused() -> void:
	var terrain := FormationTerrain.new(SIZE)
	terrain.paint(Rect2i(30, 0, 226, 32), {"depth": 3.0})
	var at := Vector2(29.5, 16.5)
	var unit: SkirmishUnit = _places.stand(_unit(), at)
	var moved := UnitMover.step(
		unit, _places.store.columns, at, Vector2(40, 16.5), 1.0, 0.1, terrain
	)
	assert_float(moved.x).is_less_equal(30.0)
	assert_float(UnitMover.share(_unit(), at, Vector2(31, 16.5), terrain)).is_less(0.000001)
