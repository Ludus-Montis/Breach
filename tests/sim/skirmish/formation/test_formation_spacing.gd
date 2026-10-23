extends GdUnitTestSuite
## Bodies apart, and one call on a threat, per spec 27 round 9 and Decision 106: routers
## caught by a friend come to rest behind it, their bodies clear of its own and of each
## other; units resting on one spot in the scrum part; and a line that has turned to meet
## one wave holds that call while it presses, rather than swinging between two.

const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


func _def(hp: int = 100) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(2)]
	unit_def.speed = 1.0
	return unit_def


func _line(count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([_def(), Vector2i(0, column)])
	return placements


## The feel test's field with its content units, the kingdom's line led by a captain.
func _captained_field(battle_seed: int) -> FormationField:
	return FormationField.new(
		0.1,
		CrateContent.unit("grem"),
		8,
		CrateContent.unit("kingdom_militia"),
		CrateContent.unit("grem_chieftain"),
		CrateContent.unit("kingdom_captain"),
		battle_seed
	)


func _cell(at: Vector2) -> Vector2i:
	return Vector2i(floori(at.x), floori(at.y))


func test_routers_caught_by_a_friend_rest_behind_it_bodies_apart() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var mine := sim.spawn_squad(4, _line(4), "player", true)
	mine.front_distance = 30.0 / 64.0
	var theirs := sim.spawn_squad(4, _line(4), "the_kingdom", false)
	theirs.front_distance = 32.0 / 64.0
	var friend := sim.spawn_squad(4, _line(4), "player", true)
	friend.front_distance = 15.0 / 64.0
	FormationMarch.sync_units(sim.squads())  # placed there
	sim.order(friend.id, SkirmishUnit.Order.HOLD)
	for _i in range(5):
		sim.step()
	mine.morale = 0
	for _i in range(45):  # they shove through the friend to rest behind it
		sim.step()

	var front: float = friend.living().map(func(u): return UnitPlaces.of(friend, u).x).min()
	var bodies := []
	for unit in friend.living():
		bodies.append(UnitBodies.at(friend, unit))
	var caught := 0
	for unit in mine.living():
		if mine.store.columns.value(unit.slot, UnitColumns.CAUGHT) > 0:
			var at := FormationRout.where(mine, unit.id)
			assert_float(at.x).is_less_equal(front)  # behind it
			bodies.append(at)
			caught += 1
	assert_int(caught).is_greater(0)
	for i in range(bodies.size()):
		for j in range(i + 1, bodies.size()):
			assert_float(bodies[i].distance_to(bodies[j])).is_greater_equal(0.99)


func test_units_resting_on_one_spot_part() -> void:
	var sim := FormationSimulation.new(2.0, 0.1)
	var squad := sim.spawn_squad(2, _line(2), "player", true)
	sim.step()
	for unit in squad.units:
		SquadEntries.loosen(squad, unit, Vector2(10.5, 10.5))  # its next step there too

	UnitBodies.step([squad], 1)

	var spots := squad.units.map(func(u): return SquadEntries.loose_at(squad, u))
	assert_float(spots[0].distance_to(spots[1])).is_equal_approx(1.0, 0.0001)


func test_a_captained_line_makes_one_call_between_two_waves() -> void:
	var field := _captained_field(2)
	for _i in range(3000):
		if field.waves["A"].built() == 8 and field.waves["B"].built() == 9:
			break
		field.step()
	field.send_together(["A", "B"])

	var faced := []
	for _i in range(600):
		for event in field.step():
			if event["type"] == "faced" and event["squad"] == field.kingdom_line.id:
				faced.append(event["facing"])

	assert_int(faced.size()).is_less_equal(1)


func test_a_captained_line_turning_to_a_flank_fills_its_front_rank() -> void:
	var field := _captained_field(680655)
	for _i in range(2000):
		if field.waves["B"].built() == 9:
			break
		field.step()
	field.send("B")
	var line := field.kingdom_line
	for _i in range(400):
		if field.step().any(func(e): return e["type"] == "faced" and e["squad"] == line.id):
			break
	for _i in range(15):
		field.step()

	assert_int(line.state).is_not_equal(SkirmishSquad.State.FIGHTING)
	for unit in line.living():  # none stuck short of its place, as one was in front of the captain
		var at := ScrumReach.at(line, unit)
		assert_float(at.distance_to(ScrumStance.anchor_one(line, unit))).is_less(0.05)
