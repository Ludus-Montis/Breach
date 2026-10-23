extends GdUnitTestSuite
## No two bodies stand on each other while marching (spec 30 round 3, part 2): two in
## their frames that overlap part (UnitBodies), staying in their frames, under either
## engine. Pouring through a gap itself is the crate's march's (march/tests.rs, spec 39
## step 3d).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const SEED := 841592
const GREM := "grem"
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


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


func test_framed_bodies_part_alike_under_either_engine() -> void:
	if not NativeKernels.available(NativeKernels.RUST):
		print("test_formation_pour: no native kernel built - native case skipped")
		return
	var reference := _framed_crowd()
	UnitBodies.step(reference, SEED)
	NativeKernels.use(NativeKernels.RUST)
	var native := _framed_crowd()
	UnitBodies.step(native, SEED)
	for index in 3:
		var want: Vector2 = UnitPlaces.of(reference[0], reference[0].units[index])
		assert_vector(UnitPlaces.of(native[0], native[0].units[index])).is_equal(want)
		assert_bool(native[0].loose.has(native[0].units[index].id)).is_false()
