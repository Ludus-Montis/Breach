extends GdUnitTestSuite
## Units hold their preferred band, per Decision 47: a back-preferring unit never steps up
## into the front rank; an enemy facing an empty front cell seeks another foe (Decision
## 88); when the whole front has fallen the squad re-anchors on its foremost rank and the
## enemy must advance to it; a squad with no front units holds once an enemy is within its
## ranged reach.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")

const ROUTE := 9.0
const TICK := 0.1


func _melee(hp: int, dmg: int, speed: float = 1.0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = speed
	return unit_def


func _spitter(hp: int = 400) -> UnitDef:
	var unit_def := _melee(hp, 0)
	unit_def.preferred_position = UnitDef.Position.BACK
	unit_def.items = [_weapon("spit", 4, 5), _weapon("claw", 1, 0), _weapon("bite", 1, 0)]
	return unit_def


func _weapon(weapon_name: String, damage: int, attack_range: int) -> WeaponDef:
	var weapon := WeaponDef.new()
	weapon.item_name = weapon_name
	weapon.damage = damage
	weapon.attack_range = attack_range
	return weapon


func _until(sim: FormationSimulation, done: Callable, limit: int = 4000) -> Array:
	var log := []
	for i in range(limit):
		log.append_array(sim.step())
		if done.call():
			break
	return log


func _steps(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for i in range(ticks):
		log.append_array(sim.step())
	return log


## How far along the route `unit` of `squad` stands, in the store (spec 35 step 7c).
func _along(squad: SkirmishSquad, unit: SkirmishUnit) -> float:
	return squad.store.columns.value(unit.slot, UnitColumns.DISTANCE)


func _militia_line(sim: FormationSimulation, count: int) -> SkirmishSquad:
	var line := []
	for column in range(count):
		line.append([_melee(400, 10, 0.8), Vector2i(0, column)])
	return sim.spawn_squad(count, line, "the_kingdom", false)


func test_a_spitter_does_not_step_into_a_fallen_front_cell() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var placements := [
		[_melee(1, 1), Vector2i(0, 0)],
		[_melee(400, 1), Vector2i(0, 1)],
		[_spitter(), Vector2i(1, 0)]
	]
	var mine := sim.spawn_squad(2, placements, "player", true)
	_militia_line(sim, 2)
	var weak: SkirmishUnit = mine.units[0]
	var spitter: SkirmishUnit = mine.units[2]

	_until(sim, func(): return not weak.is_alive())
	var where := _along(mine, spitter)
	_steps(sim, 10)

	assert_int(UnitPlace.rank(mine, spitter)).is_equal(1)
	assert_float(_along(mine, spitter)).is_equal_approx(where, 0.0001)


func test_the_enemy_facing_the_empty_cell_seeks_another_foe() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var placements := [
		[_melee(1, 1), Vector2i(0, 0)],
		[_melee(400, 1), Vector2i(0, 1)],
		[_spitter(), Vector2i(1, 0)]
	]
	var mine := sim.spawn_squad(2, placements, "player", true)
	var theirs := _militia_line(sim, 2)
	var weak: SkirmishUnit = mine.units[0]
	var opposite: SkirmishUnit = theirs.units[1]  # mirrored: it faces our column 0
	_until(sim, func(): return not weak.is_alive())

	var log := _steps(sim, 40)  # it walks round to the next foe

	var its_hits := log.filter(func(e): return e["type"] == "hit" and e["unit"] == opposite.id)
	assert_bool(its_hits.is_empty()).is_false()  # it doesn't stand idle


func test_when_the_whole_front_falls_the_enemy_must_advance_to_the_back_rank() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(
		1, [[_melee(1, 1), Vector2i(0, 0)], [_spitter(), Vector2i(1, 0)]], "player", true
	)
	var theirs := _militia_line(sim, 1)
	var spitter: SkirmishUnit = mine.units[1]
	_until(sim, func(): return not mine.units[0].is_alive())
	var where := _along(mine, spitter)

	assert_int(UnitPlace.rank(mine, spitter)).is_equal(0)  # re-anchored where it stood
	assert_float(_along(mine, spitter)).is_equal_approx(where, 0.0001)
	var log := _until(sim, func(): return mine.state == SkirmishSquad.State.FIGHTING, 200)
	log.append_array(_steps(sim, 20))

	assert_float(_along(mine, spitter)).is_equal_approx(where, 0.0001)  # it never ran forward
	var its_hits := log.filter(func(e): return e["type"] == "hit" and e["unit"] == spitter.id)
	assert_bool(its_hits.is_empty()).is_false()
	assert_int(its_hits[0]["dmg"]).is_equal(2)  # claw and bite


func test_a_squad_of_spitters_holds_once_an_enemy_is_in_range() -> void:
	var sim := FormationSimulation.new(ROUTE, TICK)
	var mine := sim.spawn_squad(
		2, [[_spitter(), Vector2i(0, 0)], [_spitter(), Vector2i(0, 1)]], "player", true
	)
	var theirs := _militia_line(sim, 2)
	sim.order(theirs.id, SkirmishUnit.Order.HOLD)
	sim.step()
	var moving := mine.state

	_until(sim, func(): return mine.state == SkirmishSquad.State.HOLDING)
	var held_at := mine.front_distance
	_steps(sim, 20)

	assert_int(moving).is_equal(SkirmishSquad.State.MOVING)
	assert_float(mine.front_distance).is_equal_approx(held_at, 0.0001)
	var gap := absf(theirs.front_distance - mine.front_distance)
	assert_float(gap).is_less_equal(5 * SkirmishSquad.RANK_DEPTH + 0.0001)
	assert_float(gap).is_greater(FormationSimulation.MELEE_REACH)
