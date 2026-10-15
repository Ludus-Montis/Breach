extends GdUnitTestSuite
## Discipline, per Decision 92 and spec 27 round 6: a formation's discipline is its units'
## mean plus its leader's bolster; a disciplined formation re-forms as a whole to meet a
## flank closing in, marching or holding, while an undisciplined one (even led) doesn't;
## and a re-form is quicker the more disciplined the formation.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


func _def(discipline: int, leadership: int = 0) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.discipline = discipline
	unit_def.leadership = leadership
	return unit_def


func _row(unit_def: UnitDef, columns: int) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([unit_def, Vector2i(0, column)])
	return placements


func _sim() -> FormationSimulation:
	var sim := FormationSimulation.new(1.0, TICK)
	return sim


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(sim.step())
	return log


func _of(log: Array, kind: String) -> Array:
	return log.filter(func(e): return e["type"] == kind)


## A kingdom line 6 wide facing west, holding at (40, 32), of these units.
func _line(sim: FormationSimulation, placements: Array) -> SkirmishSquad:
	var route := FormationRoute.new(PackedVector2Array([Vector2(40, 32), Vector2(0, 32)]))
	var line := sim.spawn_squad(6, placements, "the_kingdom", true, 0, route)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	return line


## A player squad 2 wide coming south down x = 40.5 onto the line's north side.
func _flank(sim: FormationSimulation) -> SkirmishSquad:
	var route := FormationRoute.new(PackedVector2Array([Vector2(40.5, 0), Vector2(40.5, 64)]))
	return sim.spawn_squad(2, _row(_def(30), 2), "player", true, 0, route)


func test_discipline_is_the_mean_bolstered_by_the_leader() -> void:
	var sim := _sim()
	var placements := _row(_def(30), 3)
	placements.append([_def(60, 2), Vector2i(0, 3)])

	var squad := sim.spawn_squad(4, placements, "player", true)

	assert_int(FormationDiscipline.of(squad)).is_equal(38 + 20)  # (30*3 + 60) / 4, + 2 x 10


func test_a_disciplined_line_meets_a_flank_without_a_leader() -> void:
	var sim := _sim()
	_line(sim, _row(_def(60), 6))
	_flank(sim)

	var faced := _of(_run(sim, 250), "faced")

	assert_bool(faced.is_empty()).is_false()
	assert_int(faced[0]["facing"]).is_equal(SquadFrame.NORTH)


func test_an_undrilled_line_does_not_even_when_led() -> void:
	var sim := _sim()
	var placements := _row(_def(30), 6)
	placements[0][0] = _def(30, 1)  # 30 + 10: short of re-forming as a whole
	_line(sim, placements)
	_flank(sim)

	assert_int(_of(_run(sim, 250), "faced").size()).is_equal(0)


func test_a_marching_formation_stops_to_meet_a_flank() -> void:
	var sim := _sim()
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 32), Vector2(128, 32)]))
	var column := sim.spawn_squad(4, _row(_def(60), 4), "player", true, 0, east)
	column.front_distance = 30.0 / 64.0
	var south := FormationRoute.new(PackedVector2Array([Vector2(44, 16), Vector2(44, 64)]))
	var raiders := sim.spawn_squad(2, _row(_def(30), 2), "the_kingdom", true, 0, south)

	var faced := []
	for _i in range(60):
		faced = _of(sim.step(), "faced")
		if not faced.is_empty():
			break
	assert_bool(faced.is_empty()).is_false()
	assert_int(faced[0]["squad"]).is_equal(column.id)
	assert_int(faced[0]["facing"]).is_equal(SquadFrame.NORTH)
	var halted_at := column.position.x
	_run(sim, 8)  # the raiders are still closing in: it holds to receive them
	assert_float(column.position.x).is_equal_approx(halted_at, 1.0)
	assert_bool(raiders.is_destroyed()).is_false()


## Ticks for a squad of this discipline, its units knocked 3 cells out of their places, to
## re-form.
func _reformed(discipline: int) -> int:
	var sim := _sim()
	var lane := FormationRoute.new(PackedVector2Array([Vector2(0, 20), Vector2(64, 20)]))
	var squad := sim.spawn_squad(8, _row(_def(discipline), 8), "player", true, 0, lane)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	sim.step()
	for unit in squad.living():
		var scattered := UnitPlaces.of(squad, unit) + Vector2(0, 3)
		SquadEntries.loosen(squad, unit, {"unit": unit, "next": scattered, "goal": null}, scattered)
	for tick in range(1, 400):
		sim.step()
		if not FormationScrum.regrouping(squad):
			return tick
	return 400


func test_a_re_form_is_quicker_when_disciplined() -> void:
	var drilled := _reformed(60)
	var ragged := _reformed(20)

	assert_int(drilled).is_greater(0)
	assert_int(drilled).is_less(ragged)


func test_the_steadier_a_formation_the_shorter_it_pursues() -> void:
	var leashes := []
	for discipline in [80, 75, 60, 50, 30, 25, 24, 150]:
		var sim := _sim()
		var squad := sim.spawn_squad(2, _row(_def(discipline), 2), "player", true)
		leashes.append(FormationDiscipline.pursuit_leash(squad))

	assert_array(leashes).is_equal([32.0, 32.0, 64.0, 64.0, 128.0, 128.0, INF, 32.0])


func test_a_pursuing_leader_leashes_a_step_further_and_a_cautious_one_a_step_shorter() -> void:
	var leashes := []
	for tactic in ["pursues", "cautious"]:
		for discipline in [80, 24]:
			var leader := _def(discipline)
			leader.tactics.assign([tactic])
			var sim := _sim()
			var placements := _row(_def(discipline), 1) + [[leader, Vector2i(0, 1)]]
			var squad := sim.spawn_squad(2, placements, "player", true)
			leashes.append(FormationDiscipline.pursuit_leash(squad))

	assert_array(leashes).is_equal([64.0, INF, 16.0, 128.0])


func test_a_unit_breaking_ranks_is_leashed_by_its_own_discipline() -> void:
	var sim := _sim()
	var placements := [[_def(80), Vector2i(0, 0)], [_def(10), Vector2i(0, 1)]]
	var squad := sim.spawn_squad(2, placements, "player", true)

	var leashes: Array = squad.living().map(
		func(u): return FormationDiscipline.unit_leash(squad, u)
	)
	assert_bool(leashes.has(32.0)).is_true()
	assert_bool(leashes.has(INF)).is_true()


func test_a_leader_steadies_its_units_against_breaking_ranks() -> void:
	var sim := _sim()
	var placements := [[_def(30), Vector2i(0, 0)], [_def(30, 2), Vector2i(0, 1)]]
	var led := sim.spawn_squad(2, placements, "player", true)  # a captain: leadership 2
	var unled := sim.spawn_squad(1, [[_def(30), Vector2i(0, 0)]], "the_kingdom", true)

	var ranker: SkirmishUnit = led.living()[0]
	assert_int(FormationDiscipline.unit_discipline(led, ranker)).is_equal(50)
	assert_float(FormationDiscipline.unit_leash(led, ranker)).is_equal(64.0)  # the group's
	assert_float(FormationDiscipline.unit_leash(unled, unled.living()[0])).is_equal(128.0)
