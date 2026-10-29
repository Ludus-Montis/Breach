extends GdUnitTestSuite
## The scrum's pursuits, one call into the crate over the scrum's roster
## (FormationPursuit.step, spec 39 step 8b5-xvi): a pursuer that gives up says so, as
## FormationEvents made it, and one back at its post holds the route it held it on again,
## kept by the crate as its pursuit began, GDScript's copy too.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationPursuit = preload("res://sim/skirmish/formation/formation_pursuit.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const ROAD := [Vector2(0, 32), Vector2(128, 32)]
const OWN := [Vector2(0, 40), Vector2(128, 40)]


func _one() -> Array:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return [[unit_def, Vector2i(0, 0)]]


## A pursuer on the road, pursuing squad `foe` from its post at 20 cells (returning if
## `back`), having held it on its own route, and facing the other way there.
func _pursuer(sim: FormationSimulation, foe: int, back: bool) -> SkirmishSquad:
	var road := FormationRoute.new(PackedVector2Array(ROAD), 2.0)
	var squad := sim.spawn_squad(1, _one(), "player", true, 0, road)
	var held := PackedFloat64Array([foe, 20.0 / 64.0, 20.0, 32.0, 0.0, -1, 1, 1 if back else 0])
	held.append(2.0)  # its own route's half width, then its points
	for point in OWN:
		held.append_array(PackedFloat64Array([point.x, point.y]))
	sim._field.put_squad_pursuit(squad.slot, held)
	return squad


func _step(sim: FormationSimulation) -> Array:
	var events := []
	FormationPursuit.step(sim.squads(), 4, 20.0, TICK, events, CrateRoster.of(sim.squads()))
	return events


func test_a_pursuer_whose_enemy_is_gone_gives_up_and_says_so() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var mine := _pursuer(sim, 99, false)

	var events := _step(sim)

	assert_bool(mine.pursuit["returning"]).is_true()
	var expected := {"type": "pursuit_ended", "tick": 4, "squad": mine.id, "faction": "player"}
	assert_array(events).is_equal([expected])


func test_back_at_its_post_it_holds_its_own_route_again_and_turns_there() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var mine := _pursuer(sim, 99, true)
	mine.order = SkirmishUnit.Order.ADVANCE  # its march back ended

	var events := _step(sim)

	assert_bool(mine.pursuit.is_empty()).is_true()
	assert_array(Array(mine.route.points())).is_equal(OWN)
	assert_float(mine.route.corridor_half_width).is_equal(2.0)
	assert_float(mine.position.y).is_equal(40.0)  # on its own route
	assert_array(events.map(func(e): return e["type"])).is_equal(["turning", "turned"])
