extends GdUnitTestSuite
## Both sides seek contact, per Decision 88 and spec 27 round 5: in a fight each unit
## walks to the nearest open cell next to an enemy (diagonals count), so a flank wraps and
## the far end of a flanked line comes to meet it; blows from outside a unit's front are
## flank blows; a led line turns to meet a threat before contact; a fight that can't go on
## is released rather than frozen; and when it ends units regroup and march on.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _def(hp: int, dmg: int, leadership: int = 0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = 1.0
	unit_def.leadership = leadership
	return unit_def


func _row(unit_def: UnitDef, columns: int, ranks: int = 1) -> Array:
	var placements := []
	for rank in range(ranks):
		for column in range(columns):
			placements.append([unit_def, Vector2i(rank, column)])
	return placements


func _sim() -> FormationSimulation:
	var sim := FormationSimulation.new(1.0, TICK)
	return sim


## A kingdom line `width` wide facing west, holding with its front at (40, 32); its north
## face is at y = 32 - width / 2.
func _line(sim: FormationSimulation, width: int, hp: int, led: int = 0) -> SkirmishSquad:
	var route := FormationRoute.new(PackedVector2Array([Vector2(40, 32), Vector2(0, 32)]))
	var placements := _row(_def(hp, 2), width)
	if led > 0:
		placements[0][0] = _def(hp, 2, led)
	var line := sim.spawn_squad(width, placements, "the_kingdom", true, 0, route)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	return line


## A player squad `width` wide coming south down x = 40.5 onto the line's north face.
func _from_north(sim: FormationSimulation, width: int, hp: int, dmg: int) -> SkirmishSquad:
	var route := FormationRoute.new(PackedVector2Array([Vector2(40.5, 0), Vector2(40.5, 64)]))
	return sim.spawn_squad(width, _row(_def(hp, dmg), width), "player", true, 0, route)


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(sim.step())
	return log


func _of(log: Array, kind: String) -> Array:
	return log.filter(func(e): return e["type"] == kind)


## The gap between two 1-cell units' bodies, one at `here`, one at `there`.
func _gap(here: Vector2, there: Vector2) -> float:
	var mine: SkirmishUnit = _places.stand(SkirmishUnit.new(), here)
	var theirs: SkirmishUnit = _places.stand(SkirmishUnit.new(), there)
	var a := SkirmishSquad.new(
		1, "player", 1, 0.0, 1, [mine] as Array[SkirmishUnit], null, _places.store
	)
	var b := SkirmishSquad.new(
		2, "the_kingdom", -1, 0.0, 1, [theirs] as Array[SkirmishUnit], null, _places.store
	)
	return ScrumReach.gap(a, mine, b, theirs)


func test_units_touch_face_to_face_and_at_a_slant_but_not_across_a_gap() -> void:
	var here := Vector2(0.5, 0.5)

	assert_float(_gap(here, Vector2(1.5, 0.5))).is_equal_approx(0.0, 0.0001)
	assert_float(_gap(here, Vector2(1.2, 1.2))).is_less_equal(BattleTuning.current().reach_contact)
	assert_float(_gap(here, Vector2(2.0, 0.5))).is_greater(BattleTuning.current().reach_contact)


func test_a_units_front_is_the_three_cells_ahead() -> void:
	var at := Vector2(0.5, 0.5)

	var east := UnitMotion.of_facing(SquadFrame.EAST)
	assert_bool(ScrumReach.in_front(east, at, Vector2(1.5, 0.5))).is_true()
	assert_bool(ScrumReach.in_front(east, at, Vector2(1.5, 1.5))).is_true()
	assert_bool(ScrumReach.in_front(east, at, Vector2(0.5, 1.5))).is_false()
	assert_bool(ScrumReach.in_front(east, at, Vector2(-0.5, 0.5))).is_false()


func test_contests_go_to_the_earliest_then_the_readiest() -> void:
	var quick := SkirmishUnit.new()
	quick.id = 1
	var die := BattleTuning.current().combat_contest_die
	quick.initiative = 10 + die + 1  # out-rolls any roll of the other
	var slow := SkirmishUnit.new()
	slow.id = 2
	slow.initiative = 10

	var early := ScrumContest.key(slow, 1.0, 0.0, 7, 3)
	var late := ScrumContest.key(quick, 2.0, 0.0, 7, 3)
	assert_bool(ScrumContest.before(early, late)).is_true()
	var level := ScrumContest.key(quick, 1.0, 0.0, 7, 3)
	assert_bool(ScrumContest.before(level, early)).is_true()
	assert_array(ScrumContest.key(slow, 1.0, 0.0, 7, 3)).is_equal(early)
	assert_int(ScrumContest.draw(slow, 7)).is_not_equal(ScrumContest.draw(quick, 7))


func test_a_flank_wraps_round_the_line() -> void:
	var sim := _sim()
	var line := _line(sim, 4, 400)
	var attackers := _from_north(sim, 6, 400, 1)
	var north_face := 32.0 - 2.0

	var log := _run(sim, 250)

	assert_int(_of(log, "flanked").size()).is_greater_equal(1)
	var beside := attackers.living().filter(
		func(u): return UnitPlaces.of(attackers, u).y > north_face
	)
	assert_int(beside.size()).is_greater_equal(1)
	var flank_hits := _of(log, "hit").filter(func(e): return e["flank"])
	assert_int(flank_hits.size()).is_greater(0)
	assert_int(line.state).is_equal(SkirmishSquad.State.FIGHTING)


func test_the_far_end_of_a_flanked_line_comes_to_meet_it() -> void:
	var sim := _sim()
	var line := _line(sim, 8, 400)
	_from_north(sim, 2, 400, 1)
	var far: SkirmishUnit = line.units[0]  # column 0: the south end, far from the flank
	sim.step()
	var start := UnitPlaces.of(line, far)

	var log := _run(sim, 250)

	assert_float(UnitPlaces.of(line, far).y).is_less(start.y - 2.0)
	var strikers := {}
	for hit in _of(log, "hit"):
		if hit["faction"] == "the_kingdom":
			strikers[hit["unit"]] = true
	assert_int(strikers.size()).is_greater_equal(3)


func test_a_lone_flank_fights_to_the_end() -> void:
	var sim := _sim()
	var line := _line(sim, 4, 10)
	_from_north(sim, 2, 400, 3)

	var log := _run(sim, 900)

	assert_bool(line.is_destroyed() or line.state == SkirmishSquad.State.ROUTING).is_true()
	assert_int(_of(log, "hit").size()).is_greater(0)


func test_a_led_line_turns_to_meet_a_flank_before_contact() -> void:
	var sim := _sim()
	_line(sim, 6, 400, 2)
	_from_north(sim, 2, 400, 1)

	var log := _run(sim, 250)

	var faced := _of(log, "faced")
	assert_int(faced.size()).is_greater_equal(1)
	assert_int(faced[0]["facing"]).is_equal(SquadFrame.NORTH)
	var flanked := _of(log, "flanked")
	if not flanked.is_empty():
		assert_int(faced[0]["tick"]).is_less(flanked[0]["tick"])


func test_a_leaderless_line_does_not_turn_as_a_line() -> void:
	var sim := _sim()
	_line(sim, 6, 400)
	_from_north(sim, 2, 400, 1)

	var log := _run(sim, 250)

	assert_int(_of(log, "faced").size()).is_equal(0)


func test_after_the_fight_units_regroup_and_march_on() -> void:
	var sim := _sim()
	var line := _line(sim, 1, 1)
	var attackers := _from_north(sim, 3, 400, 5)

	_run(sim, 400)

	assert_bool(line.is_destroyed()).is_true()
	assert_bool(FormationScrum.regrouping(attackers)).is_false()
	assert_float(attackers.position.y).is_greater(40.0)


func test_the_scrum_replays_the_same() -> void:
	var logs := []
	for _r in range(2):
		var sim := _sim()
		_line(sim, 4, 30)
		_from_north(sim, 6, 30, 2)
		logs.append(_run(sim, 400))

	assert_array(logs[1]).is_equal(logs[0])


func test_a_wavering_line_still_meets_its_enemy() -> void:
	var sim := _sim()
	var line := _line(sim, 8, 400)
	_from_north(sim, 2, 400, 1)
	var far: SkirmishUnit = line.units[0]  # column 0: the south end, far from the flank
	sim.step()
	var start := UnitPlaces.of(line, far)
	var log := []
	for _i in range(250):
		line.morale = 24  # wavering throughout (Decision 101), short of a rout
		log.append_array(sim.step())

	assert_float(UnitPlaces.of(line, far).y).is_less(start.y - 2.0)
	var blows: Array = _of(log, "hit").filter(func(e): return e["faction"] == "the_kingdom")
	assert_bool(blows.is_empty()).is_false()  # it lands worse: BlowLanding's morale shift
