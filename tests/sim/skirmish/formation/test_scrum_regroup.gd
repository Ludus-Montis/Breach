extends GdUnitTestSuite
## The scrum's regroup and chasers, one call into the crate over the scrum's roster
## (ScrumRegroup.step, spec 39 step 8b5-xiv): a squad out of the fight walks its loose units
## back to their places.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const ScrumRegroup = preload("res://sim/skirmish/formation/scrum_regroup.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

const TICK := 0.1


func _one() -> Array:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	return [[unit_def, Vector2i(0, 0)]]


func test_a_squad_out_of_the_fight_walks_its_loose_units_back_to_their_places() -> void:
	var sim := FormationSimulation.new(9.0, TICK)
	var mine := sim.spawn_squad(1, _one(), "player", true)
	var unit: SkirmishUnit = mine.units[0]
	var home := UnitPlaces.of(mine, unit)
	SquadEntries.loosen(mine, unit, home + Vector2(0.0, 2.0))
	var ctx := {"pace": 0.8, "seconds": TICK, "cells_per_second": 8.0}
	ctx.merge({"seed": sim.fight_seed, "terrain": null})

	ScrumRegroup.step(sim._field, CrateRoster.of(sim.squads()), ctx)

	var away := SquadEntries.loose_at(mine, unit).distance_to(home)
	assert_bool(away < 2.0).is_true()
