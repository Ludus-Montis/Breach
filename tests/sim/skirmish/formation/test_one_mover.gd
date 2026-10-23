extends GdUnitTestSuite
## Units move through one mover, which pays the ground (Decision 139, UnitMover): a router
## fleeing through a wood goes at half its open-ground pace, as a march or a walk would;
## the mover's own steps are cargo tests (native/rust/src/mover/).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const RoutFlight = preload("res://sim/skirmish/formation/rout_flight.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")

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
	_places.pace(unit, 1.0)
	return unit


## How far along its route a lone router gets in one step, on `terrain` (null: open).
func _flight(terrain) -> float:
	var squad := SkirmishSquad.new(
		1, "player", 1, 0.0, 1, [] as Array[SkirmishUnit], null, _places.store
	)
	squad.route = FormationRoute.new(PackedVector2Array([Vector2(0, 16), Vector2(256, 16)]))
	var unit: SkirmishUnit = _places.stand(_unit(), Vector2(100, 16))  # its slot in the store
	SquadEntries.flee(squad, unit, 100.0, Vector2.ZERO)
	var view := UnitView.hold([squad], FormationRout.HELD)  # as the rout holds them
	RoutFlight.step(squad, unit, 0.0, [1.0, 7, terrain, Vector2(100, 16), {}, view])
	UnitView.release(view)
	return 100.0 - squad.store.columns.value(unit.slot, UnitColumns.ALONG)


func test_a_router_in_a_wood_flees_at_half_pace() -> void:
	var open := _flight(null)
	var wood := _flight(_wood())
	assert_float(open).is_greater(0.1)
	assert_float(wood / open).is_equal_approx(0.5, 0.01)
