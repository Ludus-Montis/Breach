extends GdUnitTestSuite
## Leaders, per Decisions 81 and 87 and spec 27 round 3: a formation's leadership is its
## best living leader's, a fallen leader shakes it, and a coordinated leader waiting to
## see its partner times the departure so both arrive together.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationStaging = preload("res://sim/skirmish/formation/formation_staging.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const CHIEFTAIN := preload("res://content/units/grem_chieftain.tres")
const GREM := preload("res://content/units/grem.tres")

const TICK := 0.1


func _route(points: Array) -> FormationRoute:
	return FormationRoute.new(PackedVector2Array(points))


func _def(detection: float = 40.0, tactics: Array[String] = []) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 50
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.detection_range = detection
	unit_def.leadership = 2 if not tactics.is_empty() else 0
	unit_def.tactics = tactics
	return unit_def


func test_the_chieftain_leads_and_is_coordinated() -> void:
	assert_int(CHIEFTAIN.leadership).is_equal(3)
	assert_bool(CHIEFTAIN.tactics.has("coordinated")).is_true()
	assert_int(CHIEFTAIN.preferred_position).is_equal(UnitDef.Position.MID)


func test_a_formation_takes_its_best_living_leaders_leadership() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var placements := [[GREM, Vector2i(0, 0)], [GREM, Vector2i(0, 1)], [CHIEFTAIN, Vector2i(1, 0)]]
	var squad := sim.spawn_squad(2, placements, "player", true)

	assert_int(FormationMorale.leadership(squad)).is_equal(3)
	assert_int(squad.morale).is_equal(FormationMorale.ceiling(squad))
	assert_int(squad.morale).is_equal(roundi((60 + 60 + 70) / 3.0) + 30)


func test_a_fallen_chieftain_shakes_the_formation() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var placements := [[GREM, Vector2i(0, 0)], [CHIEFTAIN, Vector2i(1, 0)]]
	var squad := sim.spawn_squad(1, placements, "player", true)
	var before := squad.morale
	UnitHealth.put_one(squad, squad.units[1], 0)

	var log := sim.step()

	assert_bool(log.any(func(e): return e["type"] == "leader_fell")).is_true()
	assert_int(squad.morale).is_less_equal(before - 4 - 30)
	assert_int(FormationMorale.leadership(squad)).is_equal(0)


## Partner A marches east along y = 0 past the meeting point (80, 0); wave B waits at
## (70, 30), sees A from afar, and comes up x = 80 to the same point. Returns the ticks each
## reached it: [A, B].
func _meet(coordinated: bool) -> Array:
	var sim := FormationSimulation.new(4.0, TICK)
	var partner := sim.spawn_squad(
		1, [[_def(), Vector2i(0, 0)]], "player", true, 0, _route([Vector2(0, 0), Vector2(120, 0)])
	)
	var tactics: Array[String] = []
	if coordinated:
		tactics.append("coordinated")
	var wave := sim.spawn_squad(
		1,
		[[_def(120.0, tactics), Vector2i(0, 0)]],
		"player",
		true,
		0,
		_route([Vector2(70, 30), Vector2(80, 30), Vector2(80, -40)])
	)
	wave.staging = {
		"at": 0.0,
		"trigger": FormationStaging.SEES_PARTNER,
		"partner": partner.id,
		"fallback": 9999,
		"meet": Vector2(80, 0),
		"meet_cells": 40.0,
	}
	var arrived := [-1, -1]
	for _i in range(400):
		sim.step()
		if arrived[0] < 0 and partner.position.x >= 80.0 - 0.0001:
			arrived[0] = sim.tick_number()
		if arrived[1] < 0 and wave.front_distance * 64.0 >= 40.0 - 0.0001:
			arrived[1] = sim.tick_number()
	return arrived


func test_a_coordinated_leader_times_the_wave_to_arrive_with_its_partner() -> void:
	var coordinated := _meet(true)
	var eager := _meet(false)

	assert_int(absi(coordinated[1] - coordinated[0])).is_less_equal(3)
	assert_int(eager[0] - eager[1]).is_greater(10)  # without one it goes at once and is early
