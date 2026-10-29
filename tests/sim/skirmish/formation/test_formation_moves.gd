extends GdUnitTestSuite
## Re-forming around units too wide or deep to trade places with, per Decision 49: a
## front-preferring unit takes free front space sideways or diagonally if there is any;
## otherwise it passes through, and the unit it passes moves back through the formation
## to the nearest free space that fits it.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

var _grem: UnitDef
var _wide_cart: UnitDef
var _deep_cart: UnitDef


func before_test() -> void:
	_grem = _def(1, 1, UnitDef.Position.FRONT)
	_wide_cart = _def(1, 2, UnitDef.Position.BACK)
	_deep_cart = _def(2, 1, UnitDef.Position.BACK)


func _def(depth: int, width: int, position: int) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 50
	unit_def.speed = 1.0
	unit_def.footprint_depth = depth
	unit_def.footprint_width = width
	unit_def.preferred_position = position
	unit_def.position_priority = 1
	return unit_def


## A squad re-forming on its own (no enemy), stepped until it settles.
func _settle(width: int, placements: Array) -> SkirmishSquad:
	var sim := FormationSimulation.new(9.0, 0.1)
	var squad := sim.spawn_squad(width, placements, "player", true)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	squad.reforming = true
	for i in range(200):
		sim.step()
		if not squad.reforming:
			break
	return squad


func _place(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2i:
	return UnitPlace.of(squad, unit)


func _no_overlaps(squad: SkirmishSquad) -> bool:
	var cells := {}
	for unit in squad.living():
		var at := UnitPlace.of(squad, unit)
		for rank in range(at.x, at.x + unit.footprint_depth):
			for column in range(at.y, at.y + unit.footprint_width):
				if cells.has(Vector2i(rank, column)) or column >= squad.width:
					return false
				cells[Vector2i(rank, column)] = true
	return true


func test_with_free_front_space_a_grem_moves_diagonally_into_it() -> void:
	var squad := _settle(3, [[_wide_cart, Vector2i(0, 1)], [_grem, Vector2i(1, 1)]])

	assert_object(_place(squad, squad.units[1])).is_equal(Vector2i(0, 0))  # beside the cart
	assert_object(_place(squad, squad.units[0])).is_equal(Vector2i(0, 1))  # the cart didn't move
	assert_bool(_no_overlaps(squad)).is_true()


func test_without_free_space_a_grem_passes_through_a_wider_unit() -> void:
	var placements := [
		[_grem, Vector2i(0, 0)], [_wide_cart, Vector2i(0, 1)], [_grem, Vector2i(1, 1)]
	]
	var squad := _settle(3, placements)

	assert_int(UnitPlace.rank(squad, squad.units[2])).is_equal(0)
	assert_int(UnitPlace.rank(squad, squad.units[1])).is_greater(0)  # the cart went back to free space
	assert_bool(_no_overlaps(squad)).is_true()


func test_without_free_space_a_grem_passes_through_a_deeper_unit() -> void:
	var placements := [
		[_grem, Vector2i(0, 0)],
		[_deep_cart, Vector2i(0, 1)],
		[_grem, Vector2i(0, 2)],
		[_grem, Vector2i(2, 1)],
	]
	var squad := _settle(3, placements)

	assert_object(_place(squad, squad.units[3])).is_equal(Vector2i(0, 1))  # it reached the front
	assert_int(UnitPlace.rank(squad, squad.units[1])).is_greater(0)
	assert_bool(_no_overlaps(squad)).is_true()


func test_a_passing_move_takes_longer_the_further_units_travel() -> void:
	var sim := FormationSimulation.new(9.0, 0.1)
	var placements := [
		[_grem, Vector2i(0, 0)], [_wide_cart, Vector2i(0, 1)], [_grem, Vector2i(1, 1)]
	]
	var squad := sim.spawn_squad(3, placements, "player", true)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	squad.reforming = true
	sim.step()

	assert_int(squad.swaps.size()).is_equal(1)
	assert_int(squad.swaps[0]["total"]).is_greater_equal(2)  # 0.06 cells at 0.5 cells/s


func test_the_crate_holds_a_move_under_way_by_its_units_ids() -> void:
	var sim := FormationSimulation.new(9.0, 0.1)
	var placements := [
		[_grem, Vector2i(0, 0)], [_wide_cart, Vector2i(0, 1)], [_grem, Vector2i(1, 1)]
	]
	var squad := sim.spawn_squad(3, placements, "player", true)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	squad.reforming = true
	sim.step()

	var swap: Dictionary = squad.swaps[0]
	var ids := squad.units.map(func(unit): return unit.id)
	assert_bool(swap["to"].has(swap["mover"])).is_true()
	assert_array(swap["to"].keys().filter(func(id): return not ids.has(id))).is_empty()
	assert_array(swap["from"].keys()).is_equal(swap["to"].keys())
	assert_bool(FormationShuffle.swapping(squad)).is_true()
	FormationShuffle.clear(squad)
	assert_bool(FormationShuffle.swapping(squad)).is_false()
