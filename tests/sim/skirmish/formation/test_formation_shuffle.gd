extends GdUnitTestSuite
## Re-forming on reinforcement, per Decision 46: after a wave joins a fight, a unit with a
## stronger claim to a forward place (band, then priority) steps forward one rank past the
## one-deep units directly ahead in its columns. The swap takes one rank at the slowest
## involved unit's speed; the passed units keep fighting until it completes, so no cell
## is ever given up; a death cancels it.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const ROUTE := 9.0
const TICK := 0.1

var _grem: UnitDef
var _brute: UnitDef
var _spitter: UnitDef


func before_test() -> void:
	_grem = _def(400, 1, 1.0, 1, 1, UnitDef.Position.FRONT, 1)
	_brute = _def(400, 1, 0.7, 2, 2, UnitDef.Position.FRONT, 2)
	_spitter = _def(400, 1, 1.0, 1, 1, UnitDef.Position.BACK, 1)


func _def(
	hp: int, dmg: int, speed: float, depth: int, width: int, position: int, priority: int
) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = speed
	unit_def.footprint_depth = depth
	unit_def.footprint_width = width
	unit_def.preferred_position = position
	unit_def.position_priority = priority
	return unit_def


func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _steps(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for i in range(ticks):
		log.append_array(sim.step())
	return log


## `front` already locked with a durable militia line; `joining` sent after it, stepped
## until it has joined (the joining tick included).
func _joined(front: Array, joining: Array, joining_width: int) -> Array:
	var sim := FormationSimulation.new(ROUTE, TICK)
	sim.spawn_squad(3, _line(_def(400, 1, 0.8, 1, 1, 0, 1), 3), "the_kingdom", false)
	var first := sim.spawn_squad(3, front, "player", true)
	while first.state != SkirmishSquad.State.FIGHTING:
		sim.step()
	sim.spawn_squad(joining_width, joining, "player", true)
	var log := []
	while not log.any(func(e): return e["type"] == "reinforced"):
		log = sim.step()
	return [sim, first]


func _of_def(squad: SkirmishSquad, footprint_width: int) -> Array:
	return squad.living().filter(func(u): return u.footprint_width == footprint_width)


func test_a_joining_brute_passes_the_grems_ahead_after_a_slower_swap() -> void:
	var setup := _joined(_line(_grem, 3), [[_brute, Vector2i(0, 0)]], 2)
	var sim: FormationSimulation = setup[0]
	var squad: SkirmishSquad = setup[1]
	var brute: Object = _of_def(squad, 2)[0]

	_steps(sim, 8)  # one rank at 0.7 speed: 9 ticks
	var midway: int = UnitPlace.rank(squad, brute)
	_steps(sim, 1)

	assert_int(midway).is_equal(1)
	assert_int(UnitPlace.rank(squad, brute)).is_equal(0)
	var passed := squad.living().filter(func(u): return UnitPlace.rank(squad, u) == 2)
	assert_int(passed.size()).is_equal(2)  # the two grems now behind it


func test_joining_grems_pass_front_rank_spitters_in_one_grem_swap() -> void:
	var setup := _joined(_line(_spitter, 3), _line(_grem, 3), 3)
	var sim: FormationSimulation = setup[0]
	var squad: SkirmishSquad = setup[1]

	_steps(sim, 6)
	var early := squad.units.slice(3).map(func(u): return UnitPlace.rank(squad, u))
	_steps(sim, 1)  # one rank (a cell) at speed 1, crowded: 0.625 s, so 7 ticks

	assert_array(early).is_equal([1, 1, 1])
	(
		assert_array(squad.units.slice(3).map(func(u): return UnitPlace.rank(squad, u)))
		. is_equal([0, 0, 0])
	)
	(
		assert_array(squad.units.slice(0, 3).map(func(u): return UnitPlace.rank(squad, u)))
		. is_equal([1, 1, 1])
	)


func test_joining_spitters_stay_at_the_back() -> void:
	var setup := _joined(_line(_grem, 3), _line(_spitter, 3), 3)
	var sim: FormationSimulation = setup[0]
	var squad: SkirmishSquad = setup[1]

	var log := _steps(sim, 20)

	assert_bool(log.any(func(e): return e["type"] == "swapped")).is_false()
	(
		assert_array(squad.units.slice(3).map(func(u): return UnitPlace.rank(squad, u)))
		. is_equal([1, 1, 1])
	)


func test_the_front_is_held_throughout_a_swap() -> void:
	var setup := _joined(_line(_spitter, 3), _line(_grem, 3), 3)
	var sim: FormationSimulation = setup[0]
	var squad: SkirmishSquad = setup[1]

	var fronts := []
	for i in range(8):
		sim.step()
		fronts.append(squad.fighters().size())

	assert_array(fronts).is_equal([3, 3, 3, 3, 3, 3, 3, 3])


func test_a_death_cancels_a_swap_and_it_starts_again() -> void:
	var setup := _joined(_line(_grem, 3), [[_brute, Vector2i(0, 0)]], 2)
	var sim: FormationSimulation = setup[0]
	var squad: SkirmishSquad = setup[1]
	var brute: Object = _of_def(squad, 2)[0]
	_steps(sim, 4)

	UnitHealth.put(squad, squad.units[0], 0)  # a grem the brute is passing falls
	_steps(sim, 6)  # past when the first swap would have finished
	var after_cancel: int = UnitPlace.rank(squad, brute)
	_steps(sim, 10)

	assert_int(after_cancel).is_equal(1)
	assert_int(UnitPlace.rank(squad, brute)).is_equal(0)
