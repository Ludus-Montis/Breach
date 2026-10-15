extends GdUnitTestSuite
## A formation sweeps round its route's bends, per Decision 105 and spec 30 round 1: its
## frame turns towards its route's heading as it marches, no faster than its outer file
## can march the arc, without halting; it faces a slanting route's true heading; and its
## units stand in the turned frame. Per Decision 116, no unit ever moves faster than it
## can walk: the wheel slows to its outer file's pace and the inner files step shorter.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationSweep = preload("res://sim/skirmish/formation/formation_sweep.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const CELLS_PER_SECOND := 8.0
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.speed = 1.0
	return unit_def


func _line(count: int) -> Array:
	var placements := []
	for column in range(count):
		placements.append([_def(), Vector2i(0, column)])
	return placements


## A squad `width` wide on a route east 32 cells, then south; returns [sim, squad].
func _at_a_bend(width: int) -> Array:
	var sim := FormationSimulation.new(4.0, TICK)
	var bend := FormationRoute.new(
		PackedVector2Array([Vector2(0, 0), Vector2(32, 0), Vector2(32, 64)])
	)
	return [sim, sim.spawn_squad(width, _line(width), "player", true, 0, bend)]


## Ticks from its front reaching the bend until its frame faces south.
func _sweep_ticks(width: int) -> int:
	var setup := _at_a_bend(width)
	var squad: SkirmishSquad = setup[1]
	var reached := -1
	for tick in range(1, 200):
		setup[0].step()
		if reached < 0 and squad.position.y > 0.0:
			reached = tick
		if reached >= 0 and is_equal_approx(squad.heading, 180.0):
			return tick - reached
	return 200


func test_the_frame_sweeps_round_a_bend_without_halting() -> void:
	var setup := _at_a_bend(8)
	var squad: SkirmishSquad = setup[1]
	var headings := []
	for _i in range(60):
		var before := squad.front_distance
		setup[0].step()
		assert_float(squad.front_distance).is_greater(before)  # it never stops to turn
		headings.append(squad.heading)

	var between := headings.filter(func(h): return h > 90.5 and h < 179.5)
	assert_bool(between.is_empty()).is_false()  # it passed through the angles between
	assert_float(squad.heading).is_equal_approx(180.0, 0.0001)


func test_a_wider_frame_sweeps_slower_at_its_outer_files_pace() -> void:
	var narrow := _sweep_ticks(2)
	var wide := _sweep_ticks(8)

	assert_int(narrow).is_less(wide)
	# 8 wide at 8 cells a second: the outer file, 4 out, would march a quarter circle (6.3
	# cells) in 0.8 s at most; stepping on along the route too, it can't hurry, so the
	# frame slows to what it can walk (FormationWalk.share) - though the frame faces south
	# a little before its outer file has walked round, leading it by up to its slack.
	var rate := FormationSweep.rate(_at_a_bend(8)[1], CELLS_PER_SECOND)
	assert_float(90.0 / rate).is_equal_approx(PI * 8.0 / 4.0 / CELLS_PER_SECOND, 0.0001)
	assert_int(wide).is_between(11, 13)


func test_no_unit_hurries_round_a_bend_and_the_inner_files_step_shorter() -> void:
	var setup := _at_a_bend(8)
	var squad: SkirmishSquad = setup[1]
	var step := CELLS_PER_SECOND * TICK
	var inner := 0.0
	var outer := 0.0
	for _i in range(80):
		var before := squad.living().map(func(u): return UnitPlaces.of(squad, u))
		setup[0].step()
		var turning := squad.heading > 90.5 and squad.heading < 179.5
		for i in range(before.size()):
			var moved: float = UnitPlaces.of(squad, squad.living()[i]).distance_to(before[i])
			assert_float(moved).is_less_equal(step + 0.0001)  # never faster than it walks
			if turning and squad.living()[i].column == 7:
				inner += moved  # turning right, the right-hand file is on the inside
			elif turning and squad.living()[i].column == 0:
				outer += moved

	assert_float(inner).is_less(outer)


func test_a_slanting_route_is_faced_at_its_true_heading() -> void:
	var sim := FormationSimulation.new(4.0, TICK)
	var slant := FormationRoute.new(PackedVector2Array([Vector2(0, 60), Vector2(30, 0)]))
	var squad := sim.spawn_squad(4, _line(4), "player", true, 0, slant)
	sim.step()

	var heading := rad_to_deg(atan2(30.0, 60.0))  # 26.6 degrees east of north
	assert_float(squad.heading).is_equal_approx(heading, 0.0001)
	var ahead := Vector2(sin(deg_to_rad(heading)), -cos(deg_to_rad(heading)))
	var ends := squad.living().map(func(u): return UnitPlaces.of(squad, u))
	var across: Vector2 = ends[3] - ends[0]
	assert_float(absf(across.dot(ahead))).is_less(0.0001)  # its line is square to its way
	assert_float(across.length()).is_equal_approx(3.0, 0.0001)
