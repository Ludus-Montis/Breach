extends GdUnitTestSuite
## The ground's toll on every unit's step (TerrainStep): a unit walking back to its place
## after a fight, one breaking ranks to chase, and one loose in a fight all go at the share
## of their pace the ground allows - half in a wood - as a formation's walk does, and never
## step into ground they can't cross. The fight's walk pays alike under either engine.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const GREM := "grem"
const SIZE := Vector2i(256, 32)
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## Open ground, a wood everywhere (half pace), or deep water east of x 30.
func _ground(kind: String) -> FormationTerrain:
	if kind == "open":
		return null
	var terrain := FormationTerrain.new(SIZE)
	if kind == "wood":
		terrain.paint(Rect2i(Vector2i.ZERO, SIZE), {"cost": 0.5})
	else:
		terrain.paint(Rect2i(30, 0, 226, 32), {"depth": 3.0})
	return terrain


func _sim(kind: String) -> FormationSimulation:
	var sim := FormationSimulation.new(4.0, 0.1)
	sim.terrain = _ground(kind)
	return sim


func _squad(sim: FormationSimulation, count: int, faction: String, front: float) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit(GREM)
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 16), Vector2(256, 16)]))
	var squad := sim.spawn_squad(count, placements, faction, faction == "player", 0, route)
	squad.front_distance = front / 64.0
	FormationMarch.sync_units([squad])  # placed there
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	return squad


## How far a lone unit, loose `off` cells from its place, gets back towards it in a tick.
func _regroup_step(kind: String, off: Vector2) -> float:
	var sim := _sim(kind)
	var squad := _squad(sim, 1, "player", 20.0)
	sim.step()  # placed and holding
	var unit: SkirmishUnit = squad.units[0]
	var at := UnitPlaces.where(sim.store, unit) + off
	SquadEntries.loosen(squad, unit, at)
	sim.step()
	return at.distance_to(
		(
			SquadEntries.loose_at(squad, unit)
			if squad.loose.has(unit.id)
			else UnitPlaces.where(sim.store, unit)
		)
	)


func test_a_unit_regrouping_in_a_wood_walks_back_at_half_pace() -> void:
	var open := _regroup_step("open", Vector2(-4, 0))
	var wood := _regroup_step("wood", Vector2(-4, 0))
	assert_float(open).is_greater(0.1)
	assert_float(wood / open).is_equal_approx(0.5, 0.01)


## How far a unit breaking ranks to chase a foe 12 cells off gets in a tick.
func _chase_step(kind: String) -> float:
	var sim := _sim(kind)
	var mine := _squad(sim, 1, "player", 10.0)
	var foe := _squad(sim, 1, "the_kingdom", 24.0)
	sim.step()
	var unit: SkirmishUnit = mine.units[0]
	var at := UnitPlaces.where(sim.store, unit)
	SquadEntries.loosen(mine, unit, at)
	mine.chase(unit, foe.id, at, 100.0)
	sim.step()
	assert_bool(mine.chasers.has(unit.id)).is_true()
	return at.distance_to(SquadEntries.loose_at(mine, unit))


func test_a_chaser_in_a_wood_runs_at_half_pace() -> void:
	var open := _chase_step("open")
	var wood := _chase_step("wood")
	assert_float(open).is_greater(0.1)
	assert_float(wood / open).is_equal_approx(0.5, 0.01)


func test_a_chaser_never_steps_into_water_it_cant_cross() -> void:
	var sim := _sim("water")
	var mine := _squad(sim, 1, "player", 26.0)
	var foe := _squad(sim, 1, "the_kingdom", 40.0)  # across the water
	sim.step()
	var unit: SkirmishUnit = mine.units[0]
	unit.walker = TerrainWalker.grounded(unit.height)  # it can't swim
	var at := UnitPlaces.where(sim.store, unit)
	SquadEntries.loosen(mine, unit, at)
	mine.chase(unit, foe.id, at, 100.0)
	for _tick in 30:
		sim.step()
		if mine.loose.has(unit.id):
			assert_float(SquadEntries.loose_at(mine, unit).x).is_less(30.0)


## A wide line closing on a narrow one, its flanks walking round to seek it; returns [the
## mean step of their loose units that stepped, a fingerprint of where every unit stood
## each tick].
func _clash(kind: String, engine: String) -> Array:
	NativeKernels.use(engine)
	var sim := _sim(kind)
	var mine := _squad(sim, 8, "player", 30.0)
	var theirs := _squad(sim, 2, "the_kingdom", 40.0)
	sim.order(mine.id, SkirmishUnit.Order.ADVANCE)
	var stepped := [0.0, 0]
	var trace := []
	for _tick in 60:
		var before := {}
		for squad in [mine, theirs]:
			for unit in squad.living():
				if squad.loose.has(unit.id):
					before[unit] = SquadEntries.loose_at(squad, unit)
		sim.step()
		for squad in [mine, theirs]:
			for unit in squad.living():
				trace.append(
					[UnitPlaces.where(sim.store, unit), UnitPlaces.bearing_of(squad, unit)]
				)
				var step := 0.0
				if before.has(unit) and squad.loose.has(unit.id):
					step = before[unit].distance_to(SquadEntries.loose_at(squad, unit))
				if step > 0.05:
					stepped[0] += step
					stepped[1] += 1
	return [stepped[0] / maxi(stepped[1], 1), hash(var_to_str(trace))]


func test_units_loose_in_a_fight_in_a_wood_go_at_half_pace() -> void:
	var open := _clash("open", NativeKernels.GDSCRIPT)
	var wood := _clash("wood", NativeKernels.GDSCRIPT)
	assert_float(open[0]).is_greater(0.1)
	assert_float(wood[0] / open[0]).is_less(0.65)


func test_the_fight_in_a_wood_walks_alike_under_either_engine() -> void:
	if not NativeKernels.available(NativeKernels.RUST):
		print("test_terrain_toll: no native kernel built - native case skipped")
		return
	var reference := _clash("wood", NativeKernels.GDSCRIPT)
	var native := _clash("wood", NativeKernels.RUST)
	assert_int(native[1]).is_equal(reference[1])
