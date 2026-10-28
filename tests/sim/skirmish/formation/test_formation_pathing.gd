extends GdUnitTestSuite
## FormationPathing (spec 30 round 3, part 6b): a group plans as one walker - its tallest,
## swimming or climbing only if all can; where its route ahead is barred it goes round,
## keeping the way round on its command so every group of the command follows it; an open
## route makes no detour. A unit whose place is in water stands on the ford instead. The
## look's rules, and the path search's, are cargo tests (native/rust/src/pathing.rs and
## paths/, spec 39 step 3c).

const FormationPathing = preload("res://sim/skirmish/formation/formation_pathing.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _group(sim: FormationSimulation, route: FormationRoute, count: int) -> SkirmishSquad:
	var grem: UnitDef = CrateContent.unit("grem")
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, "player", true, 0, route)
	squad.state = SkirmishSquad.State.MOVING
	for unit in squad.units:  # none of them swims
		unit.walker = TerrainWalker.grounded(unit.height)
	return squad


## Moves the group's front `cells` along its route, its units standing on their places.
func _advance(squad: SkirmishSquad, cells: float) -> void:
	squad.front_distance = cells / 64.0
	for unit in squad.units:
		UnitPlaces.put(squad, unit, FormationWalk.place_of_one(squad, unit))


func _lane() -> FormationRoute:
	return FormationRoute.new(PackedVector2Array([Vector2(2, 20), Vector2(78, 20)]))


## Deep water across the lane at x 30-33, leaving the field open round its north end.
func _pond() -> FormationTerrain:
	var ground := FormationTerrain.new(Vector2i(80, 40))
	ground.paint(Rect2i(30, 8, 4, 32), {"depth": 3.0})
	return ground


func _detours(events: Array) -> int:
	return events.filter(func(event): return event["type"] == "detoured").size()


func test_the_group_walks_as_its_most_restricted_unit() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _group(sim, _lane(), 3)
	squad.units[0].walker = TerrainWalker.new(1.0, 2, 1, true, true)
	squad.units[1].walker = TerrainWalker.new(2.0, 1, 3, true, false)
	squad.units[2].walker = TerrainWalker.new(1.5, 3, 0, true, true)

	var walker := FormationPathing.group_walker(squad)

	assert_float(walker.height).is_equal(2.0)
	assert_int(walker.swimmer).is_equal(1)
	assert_int(walker.climber).is_equal(0)
	assert_bool(walker.swims).is_true()
	assert_bool(walker.climbs).is_false()


func test_a_barred_route_ahead_makes_a_detour_kept_on_the_command() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var ground := _pond()
	var squad := _group(sim, _lane(), 2)
	_advance(squad, 20.0)
	var events := []

	FormationPathing.step(sim.squads(), ground, 1, 0, events)
	assert_int(_detours(events)).is_equal(1)
	assert_int(squad.command.patch_count()).is_equal(1)

	FormationPathing.step(sim.squads(), ground, 2, 0, events)
	var walker := FormationPathing.group_walker(squad)
	var route := squad.route
	assert_float(route.distance_of(squad.position)).is_equal_approx(20.0, 1.0)
	var along := 0.0
	while along < route.length_cells():
		(
			assert_float(ground.crossing(walker, route.point_at(along), route.point_at(along)))
			. is_greater(0.0)
		)
		along += 0.5


func test_every_group_of_the_command_follows_the_patched_route() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var ground := _pond()
	var first := _group(sim, _lane(), 2)
	var second := _group(sim, _lane(), 2)
	second.command = first.command
	second.state = SkirmishSquad.State.HOLDING  # it doesn't look for itself
	_advance(first, 20.0)
	var events := []

	FormationPathing.step(sim.squads(), ground, 1, 0, events)
	FormationPathing.step(sim.squads(), ground, 2, 0, events)

	assert_int(second.patched_count).is_equal(1)
	assert_object(second.route).is_not_same(first.command.route)
	assert_float(second.route.length_cells()).is_greater(first.command.route.length_cells())


func test_an_open_route_makes_no_detour() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var squad := _group(sim, _lane(), 2)
	_advance(squad, 20.0)
	var events := []

	FormationPathing.step(sim.squads(), FormationTerrain.new(Vector2i(80, 40)), 1, 0, events)

	assert_int(_detours(events)).is_equal(0)
	assert_int(squad.command.patch_count()).is_equal(0)


func test_a_group_looks_again_only_once_it_has_moved_on() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var ground := _pond()
	var squad := _group(sim, _lane(), 2)
	_advance(squad, 20.0)
	var events := []
	FormationPathing.step(sim.squads(), FormationTerrain.new(Vector2i(80, 40)), 1, 0, events)

	FormationPathing.step(sim.squads(), ground, 2, 0, events)  # still where it looked
	assert_int(_detours(events)).is_equal(0)


func test_a_unit_whose_place_is_in_deep_water_stands_on_the_ford() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var ground := FormationTerrain.new(Vector2i(64, 32))
	ground.paint(Rect2i(0, 0, 64, 32), {"depth": 3.0})  # deep water it could swim
	ground.paint(Rect2i(0, 0, 64, 2), {"depth": 0.4})  # and a ford along the route
	var route := FormationRoute.new(PackedVector2Array([Vector2(0, 1), Vector2(64, 1)]))
	var squad := _group(sim, route, 4)
	for unit in squad.units:
		unit.walker = TerrainWalker.new(unit.height)  # swims, at its implicit level
	_advance(squad, 20.0)

	for unit in squad.units:
		var target := FormationWalk.target_of(squad, unit, ground)
		var mode := ground.mode(unit.walker, target, target)
		assert_int(mode).is_equal(TerrainWalker.Mode.WALKING)
