extends GdUnitTestSuite
## Morale, per Decision 82 and spec 27 round 3: a ceiling from courage and leadership;
## impact shock from side and rear hits; pressure from fighting on several sides, less
## where a friend covers a side; recovery out of contact; bands, the slower strikes of a
## wavering squad, and the shock of a fallen leader.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, column: int, leadership: int = 0) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.set_place(unit, Vector2i(0, column))
	_places.health(unit, 100)
	unit.courage = 60
	unit.leadership = leadership
	return unit


func _squad(squad_id: int, units: Array, faction: String = "player") -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	members.assign(units)
	return SkirmishSquad.new(
		squad_id, faction, 1, 0.0, maxi(1, units.size()), members, null, _places.store
	)


func _def(hp: int, dmg: int) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = 1.0
	return unit_def


func _block(unit_def: UnitDef, ranks: int, columns: int) -> Array:
	var placements := []
	for rank in range(ranks):
		for column in range(columns):
			placements.append([unit_def, Vector2i(rank, column)])
	return placements


func _route(points: Array) -> FormationRoute:
	return FormationRoute.new(PackedVector2Array(points))


## A kingdom line holding at (40, 32) facing west, and a raider coming at it from `from`.
func _raid(from: Array) -> Array:
	var sim := FormationSimulation.new(2.0, TICK)
	var route := _route([Vector2(40, 32), Vector2(0, 32)])
	var line := sim.spawn_squad(4, _block(_def(500, 1), 1, 4), "the_kingdom", true, 0, route)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	sim.spawn_squad(2, _block(_def(500, 1), 1, 2), "player", true, 0, _route(from))
	return [sim, line]


func _until(sim: FormationSimulation, kind: String) -> Array:
	var log := []
	for _i in range(400):
		var events := sim.step()
		log.append_array(events)
		if events.any(func(e): return e["type"] == kind):
			break
	return log


func test_the_ceiling_is_mean_courage_plus_leadership() -> void:
	var squad := _squad(1, [_unit(1, 0), _unit(2, 1, 2)])

	assert_int(FormationMorale.ceiling(squad)).is_equal(80)
	assert_int(FormationMorale.leadership(squad)).is_equal(2)
	squad.units[1].leadership = 9
	assert_int(FormationMorale.ceiling(squad)).is_equal(100)


func test_a_side_hit_is_a_shock() -> void:
	var setup := _raid([Vector2(41, 0), Vector2(41, 64)])
	var line: SkirmishSquad = setup[1]

	_until(setup[0], "flanked")

	assert_int(line.morale).is_equal(60 - BattleTuning.current().morale_side_impact)


func test_a_rear_hit_is_a_bigger_shock() -> void:
	var setup := _raid([Vector2(90, 32), Vector2(0, 32)])
	var line: SkirmishSquad = setup[1]

	_until(setup[0], "flanked")

	assert_int(line.morale).is_equal(60 - BattleTuning.current().morale_rear_impact)


func test_pressure_grows_with_the_sides_fought_on() -> void:
	var squad := _squad(1, [_unit(1, 0)])
	squad.engaged_with = 99
	squad.flank_contacts[SquadEdges.RIGHT] = {"foe": 98, "since": 0}
	var events := []

	FormationMorale.step([squad], 10, 10, events)
	assert_int(squad.morale).is_equal(60 - 3)
	squad.flank_contacts[SquadEdges.REAR] = {"foe": 97, "since": 0}
	FormationMorale.step([squad], 20, 10, events)
	assert_int(squad.morale).is_equal(60 - 3 - 9)


func test_a_friend_beside_a_fought_side_takes_its_pressure() -> void:
	var squad := _squad(1, [_unit(1, 0)])
	squad.engaged_with = 99
	squad.flank_contacts[SquadEdges.RIGHT] = {"foe": 98, "since": 0}
	var friend := _squad(2, [_unit(2, 0)])
	friend.front_distance = 0.0
	friend.centre_shift = 1.5  # one cell along: beside the squad's right (south) face

	FormationMorale.step([squad, friend], 10, 10, [])

	assert_int(squad.morale).is_equal(60)


func test_out_of_contact_morale_recovers_to_its_ceiling() -> void:
	var squad := _squad(1, [_unit(1, 0, 1)])
	squad.morale = 40

	FormationMorale.step([squad], 10, 10, [])
	assert_int(squad.morale).is_equal(43)
	squad.morale = 69
	FormationMorale.step([squad], 20, 10, [])
	assert_int(squad.morale).is_equal(70)


func test_crossing_a_band_is_an_event() -> void:
	var squad := _squad(1, [_unit(1, 0)])
	var events := []

	FormationMorale.shock(squad, 15, 3, events)

	assert_int(events.size()).is_equal(1)
	assert_str(events[0]["band"]).is_equal("SHAKEN")
	assert_int(FormationMorale.band(squad)).is_equal(FormationMorale.Band.SHAKEN)


func test_a_fallen_leader_shocks_the_squad_and_lowers_its_ceiling() -> void:
	var leader := _unit(2, 1, 2)
	var squad := _squad(1, [_unit(1, 0), leader])
	squad.morale = FormationMorale.ceiling(squad)
	assert_int(squad.morale).is_equal(80)
	UnitState.put(squad, leader, SkirmishUnit.State.DEAD)
	var events := []

	FormationMorale.losses(squad, [leader], 5, events)

	assert_int(squad.morale).is_equal(80 - 4 - 20)
	assert_bool(events.any(func(e): return e["type"] == "leader_fell")).is_true()
	assert_int(FormationMorale.ceiling(squad)).is_equal(60)


func test_a_wavering_squad_strikes_more_slowly() -> void:
	var squad := _squad(1, [_unit(1, 0)])
	squad.morale = 10

	assert_int(FormationMorale.interval(squad, 10)).is_equal(15)
	squad.morale = 40
	assert_int(FormationMorale.interval(squad, 10)).is_equal(10)


func test_a_frontal_fight_between_equals_does_not_break_either_side() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var mine := sim.spawn_squad(8, _block(_def(20, 6), 1, 8), "player", true)
	var theirs := sim.spawn_squad(8, _block(_def(20, 6), 1, 8), "the_kingdom", false)

	for _i in range(600):
		sim.step()

	assert_int(mine.morale).is_greater(0)
	assert_int(theirs.morale).is_greater(0)
