extends GdUnitTestSuite
## Learned detours (spec 30): a detour that worked is kept on the command as a patch to its
## route - between two distances along it, follow these waypoints - and every group of the
## command follows the patched route. A pathfinder forgets what it saw once its group is
## back on the route. The patches' own rules are cargo tests (native/rust/src/route/).

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const RoutePatch = preload("res://sim/skirmish/formation/route_patch.gd")
const PathMemory = preload("res://sim/skirmish/formation/path_memory.gd")
const RouteDetour = preload("res://sim/skirmish/formation/route_detour.gd")


func _straight() -> FormationRoute:
	return FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(40, 10)]), 3.0)


func test_a_pathfinder_forgets_what_it_saw_once_back_on_the_route() -> void:
	var memory := PathMemory.new()
	memory.learn(5, 100)

	assert_bool(memory.knows(5, 100)).is_true()
	memory.forget()
	assert_bool(memory.knows(5, 100)).is_false()


func test_a_group_back_on_its_route_ahead_keeps_its_detour_and_forgets() -> void:
	var route := _straight()
	var detour := RouteDetour.new()
	detour.memory.learn(7, 100)

	assert_object(detour.note(route, Vector2(10, 11))).is_null()  # within the corridor
	assert_object(detour.note(route, Vector2(14, 18))).is_null()  # off it
	assert_object(detour.note(route, Vector2(24, 18))).is_null()
	var patch := detour.note(route, Vector2(30, 12))  # back on it, ahead

	assert_object(patch).is_not_null()
	assert_float(patch.from_along).is_equal_approx(10.0, 0.0001)
	assert_float(patch.to_along).is_equal_approx(30.0, 0.0001)
	assert_array(patch.waypoints).is_equal(PackedVector2Array([Vector2(14, 18), Vector2(24, 18)]))
	assert_bool(detour.memory.knows(7, 100)).is_false()
	assert_object(detour.note(route, Vector2(32, 10))).is_null()  # on it, nothing new


func test_coming_back_behind_where_it_left_keeps_no_patch_but_still_forgets() -> void:
	var route := _straight()
	var detour := RouteDetour.new()
	detour.note(route, Vector2(20, 10))
	detour.note(route, Vector2(18, 16))
	detour.memory.learn(3, 100)

	assert_object(detour.note(route, Vector2(15, 10))).is_null()
	assert_bool(detour.memory.knows(3, 100)).is_false()


func test_every_group_of_the_command_follows_the_patched_route() -> void:
	var route := _straight()
	var first_group := RouteDetour.new()
	var patches := []  # the command's (FormationCommand will hold them)
	for at in [Vector2(10, 10), Vector2(15, 20), Vector2(25, 20), Vector2(30, 10)]:
		var learned := first_group.note(route, at)
		if learned != null:
			patches.append(learned)
	var followed := RoutePatch.patched(route, patches)

	assert_int(patches.size()).is_equal(1)
	# a group behind, still short of the stretch, walks the detour when it gets there
	assert_float(followed.distance_of(Vector2(4, 10))).is_equal_approx(4.0, 0.0001)
	assert_object(followed.point_at(10.0 + Vector2(5, 10).length())).is_equal(Vector2(15, 20))
	assert_object(followed.point_at(followed.length_cells())).is_equal(Vector2(40, 10))
