extends GdUnitTestSuite
## Spreading, per Decision 51 and specs/22-formation-feel-test.md (round 6): in a fight,
## front-preferring units that joined as reinforcements take free front places beyond the
## squad's columns, widening the line up to the lane's combat width. The front already in
## place stays put on screen, and a wave on its own never spreads.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const ROUTE := 9.0
const TICK := 0.1


func _def(speed: float) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = speed
	return unit_def


func _block(unit_def: UnitDef, ranks: int, width: int) -> Array:
	var placements := []
	for rank in range(ranks):
		for column in range(width):
			placements.append([unit_def, Vector2i(rank, column)])
	return placements


func _run(sim: FormationSimulation, done: Callable, limit: int = 4000) -> Array:
	var log := []
	for i in range(limit):
		log.append_array(sim.step())
		if done.call():
			break
	return log


func _settled(squad: SkirmishSquad) -> bool:
	return not squad.reforming and squad.swaps.is_empty()


## A 2x2 grem wave locked with a durable militia line on a lane `combat_width` wide.
func _fighting_block(combat_width: int, ranks: int, width: int) -> Array:
	var sim := FormationSimulation.new(ROUTE, TICK)
	sim.combat_width = combat_width
	sim.spawn_squad(3, _block(_def(0.8), 1, 3), "the_kingdom", false)
	var first := sim.spawn_squad(width, _block(_def(1.0), ranks, width), "player", true)
	_run(sim, func(): return first.state == SkirmishSquad.State.FIGHTING)
	return [sim, first]


func _joined_and_settled(combat_width: int) -> Array:
	var setup := _fighting_block(combat_width, 2, 2)
	var sim: FormationSimulation = setup[0]
	var first: SkirmishSquad = setup[1]
	sim.spawn_squad(2, _block(_def(1.0), 2, 2), "player", true)
	_run(sim, func(): return first.living().size() == 8 and _settled(first))
	return setup


func test_reinforcements_spread_into_free_combat_width() -> void:
	var setup := _joined_and_settled(5)
	var first: SkirmishSquad = setup[1]

	assert_int(first.width).is_equal(5)
	assert_int(first.fighters().size()).is_equal(5)


func test_the_front_already_in_place_stays_where_it_was() -> void:
	var setup := _fighting_block(5, 2, 2)
	var sim: FormationSimulation = setup[0]
	var first: SkirmishSquad = setup[1]
	var front := first.fighters()
	var spans_before := front.map(func(u): return first.lateral_span(u))
	sim.spawn_squad(2, _block(_def(1.0), 2, 2), "player", true)

	_run(sim, func(): return first.living().size() == 8 and _settled(first))

	assert_array(front.map(func(u): return first.lateral_span(u))).is_equal(spans_before)
	assert_array(front.map(func(u): return UnitPlace.rank(first, u))).is_equal([0, 0])


func test_units_left_behind_close_up_to_the_front() -> void:
	var setup := _joined_and_settled(5)
	var first: SkirmishSquad = setup[1]

	var ranks := first.living().map(func(u): return UnitPlace.rank(first, u))
	ranks.sort()
	assert_array(ranks).is_equal([0, 0, 0, 0, 0, 1, 1, 2])  # straight forward, no gaps


func test_the_line_stays_on_the_lane() -> void:
	var setup := _joined_and_settled(5)
	var first: SkirmishSquad = setup[1]

	for unit in first.living():
		var span := first.lateral_span(unit)
		assert_float(span.x).is_greater_equal(-3.0)  # half the lane, plus half a column
		assert_float(span.y).is_less_equal(3.0)


func test_a_wide_lane_takes_every_grem_into_the_front() -> void:
	var setup := _joined_and_settled(8)
	var first: SkirmishSquad = setup[1]

	assert_int(first.width).is_equal(8)
	assert_int(first.fighters().size()).is_equal(8)


func test_a_painted_deep_wave_stays_deep() -> void:
	var setup := _fighting_block(5, 3, 1)
	var sim: FormationSimulation = setup[0]
	var first: SkirmishSquad = setup[1]

	_run(sim, func(): return false, 60)

	assert_int(first.width).is_equal(1)
	assert_int(first.fighters().size()).is_equal(1)


func test_without_a_combat_width_reinforcements_keep_the_squads_columns() -> void:
	var setup := _joined_and_settled(0)
	var first: SkirmishSquad = setup[1]

	assert_int(first.width).is_equal(2)
	assert_int(first.fighters().size()).is_equal(2)
