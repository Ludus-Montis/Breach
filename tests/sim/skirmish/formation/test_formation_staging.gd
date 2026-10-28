extends GdUnitTestSuite
## Hold-until orders, per Decision 87: a staged wave holds at its staging point until it
## detects what its trigger asks for (its partner, or its partner fighting), or until its
## fallback runs out; it never acts on what it can't detect.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationStaging = preload("res://sim/skirmish/formation/formation_staging.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1


func _def(hp: int = 100) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(2)]
	unit_def.speed = 1.0
	return unit_def


func _line(unit_def: UnitDef, count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _route(points: Array) -> FormationRoute:
	return FormationRoute.new(PackedVector2Array(points))


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(sim.step())
	return log


func _of(log: Array, kind: String) -> Array:
	return log.filter(func(e): return e["type"] == kind)


## A kingdom line holding at x = 60 across y = 0, and the player's wave A marching at it.
func _scene(sim: FormationSimulation) -> SkirmishSquad:
	var line := sim.spawn_squad(
		4, _line(_def(), 4), "the_kingdom", true, 0, _route([Vector2(60, 0), Vector2(0, 0)])
	)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	return sim.spawn_squad(
		4, _line(_def(), 4), "player", true, 0, _route([Vector2(0, 0), Vector2(100, 0)])
	)


## Wave B on a parallel route at y = 20, staged `at` cells along it.
func _staged(sim: FormationSimulation, at: float, staging: Dictionary) -> SkirmishSquad:
	var wave := sim.spawn_squad(
		4, _line(_def(), 4), "player", true, 0, _route([Vector2(0, 20), Vector2(100, 20)])
	)
	var order := staging.duplicate()
	order["at"] = at
	wave.staging = order  # the crate holds it: written whole
	return wave


func test_a_staged_wave_holds_at_its_point() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var wave := _staged(sim, 20.0, {"trigger": FormationStaging.SEES_FIGHT, "fallback": 9999})

	var log := _run(sim, 100)

	assert_int(_of(log, "staged").size()).is_equal(1)
	assert_int(wave.state).is_equal(SkirmishSquad.State.HOLDING)
	assert_float(wave.position.x).is_between(20.0, 21.0)


func test_it_goes_when_it_sees_its_partner_engage() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var partner := _scene(sim)
	var wave := _staged(
		sim, 40.0, {"trigger": FormationStaging.SEES_FIGHT, "partner": partner.id, "fallback": 9999}
	)

	var log := _run(sim, 200)

	var engaged: int = _of(log, "engaged")[0]["tick"]
	var signalled: Array = _of(log, "signalled")
	assert_int(signalled.size()).is_equal(1)
	assert_int(signalled[0]["tick"]).is_greater_equal(engaged)
	assert_float(wave.position.x).is_greater(45.0)


func test_it_does_not_go_on_a_fight_it_cannot_see() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var partner := _scene(sim)
	var wave := _staged(
		sim, 5.0, {"trigger": FormationStaging.SEES_FIGHT, "partner": partner.id, "fallback": 9999}
	)

	var log := _run(sim, 200)

	assert_int(_of(log, "engaged").size()).is_greater_equal(1)
	assert_int(_of(log, "signalled").size()).is_equal(0)
	assert_int(wave.state).is_equal(SkirmishSquad.State.HOLDING)


func test_it_goes_when_it_sees_its_partner() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var wave := _staged(sim, 10.0, {"trigger": FormationStaging.SEES_PARTNER, "fallback": 9999})
	var partner := sim.spawn_squad(
		4, _line(_def(), 4), "player", true, 0, _route([Vector2(-80, 0), Vector2(100, 0)])
	)
	var order := wave.staging
	order["partner"] = partner.id
	wave.staging = order

	var log := _run(sim, 200)

	assert_int(_of(log, "signalled").size()).is_equal(1)
	assert_float(wave.position.x).is_greater(12.0)


func test_when_the_fallback_runs_out_it_goes_or_turns_back() -> void:
	for then in ["go", "back"]:
		var sim := FormationSimulation.new(2.0, TICK)
		var wave := _staged(
			sim, 10.0, {"trigger": FormationStaging.SEES_FIGHT, "fallback": 30, "then": then}
		)

		var log := _run(sim, 80)

		assert_int(_of(log, "gave_up").size()).is_equal(1)
		if then == "go":
			assert_float(wave.position.x).is_greater(12.0)
		else:
			assert_int(_of(log, "returned").size()).is_equal(1)  # it turned back home


func test_the_crate_holds_a_squad_s_staging_order() -> void:
	var sim := FormationSimulation.new(2.0, TICK)
	var wave := _staged(sim, 10.0, {"trigger": FormationStaging.SEES_FIGHT, "fallback": 30})
	var order := {
		"at": 12.5,
		"trigger": FormationStaging.SEES_PARTNER,
		"partner": 7,
		"fallback": 40,
		"then": "back",
		"meet": Vector2(80, 4),
		"meet_cells": 40.0,
	}
	wave.staging = order
	assert_dict(wave.staging).is_equal(order)
	wave.staging = {}
	assert_bool(wave.staging.is_empty()).is_true()
