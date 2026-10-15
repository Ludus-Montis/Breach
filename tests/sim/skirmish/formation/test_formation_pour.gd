extends GdUnitTestSuite
## Pouring through a gap (FormationPour, spec 30 round 3, part 2): units whose places lie
## in water they can't walk squeeze in to the nearest ground they can, and where those
## points fall on one another, or on a place another unit holds, they file in behind it -
## front first, then nearest the centre line - so no two make for one point. And no two
## bodies stand on each other while marching: two in their frames that overlap part
## (UnitBodies), staying in their frames, under either engine.

const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitBodies = preload("res://sim/skirmish/formation/unit_bodies.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const SEED := 841592
const GREM := "res://content/units/grem.tres"
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## A line of `count` grems that can't swim, its front in a stream 4 cells across (x 20 to
## 24) with a 4-cell ford (y 14 to 18) on its route along y 16.
func _at_the_ford(count: int) -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var terrain := FormationTerrain.new(Vector2i(64, 32))
	terrain.paint(Rect2i(20, 0, 4, 32), {"depth": 3.0})
	terrain.paint(Rect2i(20, 14, 4, 4), {"depth": 0.0})
	var grem: UnitDef = load(GREM)
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, "player", true)
	squad.order = SkirmishUnit.Order.HOLD
	squad.route = FormationRoute.new(PackedVector2Array([Vector2(0, 16), Vector2(64, 16)]))
	squad.front_distance = 22.5 / 64.0
	for unit in squad.units:
		unit.walker = TerrainWalker.grounded(unit.height)
	return [squad, terrain]


func test_units_squeezed_at_a_ford_make_for_points_a_body_apart() -> void:
	var made := _at_the_ford(8)
	var squad: SkirmishSquad = made[0]
	var terrain: FormationTerrain = made[1]
	var units := squad.living()
	var aims := units.map(func(unit): return FormationWalk.target_of(squad, unit, terrain))
	for i in units.size():
		assert_float(terrain.factor(units[i], aims[i], aims[i])).is_greater(0.0)
		for j in range(i + 1, units.size()):
			assert_float(aims[i].distance_to(aims[j])).is_greater_equal(1.0 - 0.0001)


func test_the_squeezed_file_in_behind_front_first_then_nearest_the_centre_line() -> void:
	var made := _at_the_ford(8)
	var squad: SkirmishSquad = made[0]
	var ahead := UnitMotion.vector(squad.heading)
	var across := ahead.orthogonal()
	var by_offset := {}  # how far across the frame a unit's place lies -> its target
	for unit in squad.living():
		var off := (FormationWalk.place_of(squad, unit) - squad.position).dot(across)
		by_offset[roundi(off * 2.0)] = FormationWalk.target_of(squad, unit, made[1])
	for side in [-1, 1]:  # 2.5 and 3.5 cells out squeeze onto one point: the nearer leads
		var nearer: Vector2 = by_offset[5 * side]
		var further: Vector2 = by_offset[7 * side]
		assert_float((nearer - further).dot(ahead)).is_greater_equal(1.0 - 0.0001)


func test_a_unit_whose_place_it_can_walk_keeps_to_its_place() -> void:
	var made := _at_the_ford(4)  # 4 wide: every place lies in the ford
	var squad: SkirmishSquad = made[0]
	for unit in squad.living():
		var target := FormationWalk.target_of(squad, unit, made[1])
		assert_vector(target).is_equal(FormationWalk.place_of(squad, unit))


## Two squads' framed units: two of one line on each other, a third half overlapping.
func _framed_crowd() -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var grem: UnitDef = load(GREM)
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
