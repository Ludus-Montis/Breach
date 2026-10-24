extends GdUnitTestSuite
## No two bodies stand on each other while marching (spec 30 round 3, part 2): two in
## their frames that overlap part (UnitBodies), staying in their frames. Pouring through a
## gap itself is the crate's march's (march/tests.rs, spec 39 step 3d).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const SEED := 841592
const GREM := "grem"
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## Two squads' framed units: two of one line on each other, a third half overlapping.
func _framed_crowd() -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var grem: UnitDef = CrateContent.unit(GREM)
	var placements := []
	for column in 3:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(3, placements, "player", true)
	var first := UnitPlaces.of(squad, squad.units[0])
	UnitPlaces.put(squad, squad.units[1], first)
	UnitPlaces.put(squad, squad.units[2], first + Vector2(0.0, 0.5))
	return [squad]


func test_two_framed_bodies_on_each_other_part_and_keep_to_their_frame() -> void:
	var squads := _framed_crowd()
	var squad: SkirmishSquad = squads[0]
	UnitBodies.step(squads, SEED)
	var units := squad.living()
	for i in units.size():
		assert_bool(squad.loose.has(units[i].id)).is_false()  # still in its frame
		for j in range(i + 1, units.size()):
			var apart := UnitPlaces.of(squad, units[i]).distance_to(UnitPlaces.of(squad, units[j]))
			assert_float(apart).is_greater(0.5)
