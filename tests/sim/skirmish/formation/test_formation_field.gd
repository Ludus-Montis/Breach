extends GdUnitTestSuite
## FormationField, per Decisions 86 and 87 and spec 27 rounds 1 and 2: one simulation over
## a 128 x 64 cell field, the player's waves on routes A (straight at the line) and B
## (through the wood and onto the line's north side), and the kingdom's line holding
## across A. B's wave can wait in the wood until it sees A's engage, or both can be sent
## timed to arrive together.

const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1


func _def(hp: int, dmg: int) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(dmg)]
	unit_def.speed = 1.0
	unit_def.build_seconds = 0.1
	return unit_def


func _field(kingdom_hp: int = 60, wave_hp: int = 20) -> FormationField:
	return FormationField.new(TICK, _def(wave_hp, 3), 8, _def(kingdom_hp, 2))


func _run(field: FormationField, done: Callable, limit: int = 3000) -> Array:
	var log := []
	for _i in range(limit):
		log.append_array(field.step())
		if done.call(log):
			break
	return log


func _has(kind: String) -> Callable:
	return func(log): return log.any(func(e): return e["type"] == kind)


func test_the_kingdom_holds_a_line_across_route_a() -> void:
	var field := _field()
	field.step()

	var line := field.kingdom_line
	assert_vector(line.position).is_equal_approx(
		Vector2(FormationField.LINE_AT, 32), Vector2(0.01, 0.01)
	)
	assert_int(line.facing).is_equal(SquadFrame.WEST)
	assert_int(line.state).is_equal(SkirmishSquad.State.HOLDING)
	assert_int(line.living().size()).is_equal(
		FormationField.KINGDOM_WIDTH * FormationField.KINGDOM_RANKS
	)


func test_route_a_runs_at_the_line_and_route_b_onto_its_side() -> void:
	var field := _field()

	var a = field.routes["A"]
	var b = field.routes["B"]
	assert_vector(a.point_at(a.length_cells())).is_equal(Vector2(128, 32))
	assert_vector(b.point_at(b.length_cells())).is_equal(Vector2(FormationField.FLANK_X, 64))
	assert_bool(FormationField.WOOD.has_point(FormationField.STAGING)).is_true()


func test_a_wave_sent_down_route_a_fights_the_line() -> void:
	var field := _field()
	_run(field, _has("wave_full"))

	field.send("A")
	_run(field, _has("engaged"))

	assert_int(field.kingdom_line.state).is_equal(SkirmishSquad.State.FIGHTING)


func test_a_wave_on_route_b_strikes_the_lines_side() -> void:
	var field := _field(400, 200)
	_run(field, func(log): return field.waves["B"].built() == 8)

	field.send("B")
	var log := _run(field, _has("flanked"))

	var flanked: Array = log.filter(func(e): return e["type"] == "flanked")
	assert_int(flanked.size()).is_equal(1)
	assert_int(flanked[0]["squad"]).is_equal(field.kingdom_line.id)
	var wave: SkirmishSquad = field.sim.squad(flanked[0]["by"])
	assert_float(wave.heading).is_equal_approx(180.0, 1.0)  # swept round its bend: south


func test_a_wider_waves_overhanging_units_fight_the_lines_corners() -> void:
	var field := _field(400, 200)
	_run(field, _has("wave_full"))

	field.send("A")
	var log := _run(field, func(_log): return false, 400)

	var strikers := {}
	for hit in log.filter(func(e): return e["type"] == "hit" and e["faction"] == "player"):
		strikers[hit["unit"]] = true
	# more strike than the line's 6-wide face: the ends reach its corners (Decision 88)
	assert_int(strikers.size()).is_greater(FormationField.KINGDOM_WIDTH)


func test_b_waits_in_the_wood_until_it_sees_a_engage_then_flanks() -> void:
	var field := _field(400, 200)
	field.set_wait(true)
	_run(field, func(log): return field.waves["A"].built() == 8 and field.waves["B"].built() == 8)
	field.send("B")
	var waiting := _run(field, _has("staged"))
	assert_bool(waiting.any(func(e): return e["type"] == "staged")).is_true()

	field.send("A")
	var log := _run(field, func(log): return _has("flanked").call(log) or _has("routed").call(log))

	var kinds: Array = log.map(func(e): return e["type"])
	assert_int(kinds.find("signalled")).is_greater(kinds.find("engaged"))
	assert_int(maxi(kinds.find("flanked"), kinds.find("routed"))).is_greater(
		kinds.find("signalled")
	)


func test_waves_sent_together_reach_the_line_together() -> void:
	var field := _field(400, 200)
	_run(field, func(log): return field.waves["A"].built() == 8 and field.waves["B"].built() == 8)

	field.send_together(["A", "B"])
	var log := _run(
		field, func(log): return _has("engaged").call(log) and _has("flanked").call(log)
	)

	var engaged: int = log.filter(func(e): return e["type"] == "engaged")[0]["tick"]
	var flanked: int = log.filter(func(e): return e["type"] == "flanked")[0]["tick"]
	assert_int(absi(engaged - flanked)).is_less_equal(3)  # timed by rehearsal


func test_waves_depart_on_their_own_when_set_to() -> void:
	var field := _field()
	field.set_auto("A", true)

	var log := _run(field, _has("departed"))

	assert_bool(log.any(func(e): return e["type"] == "departed")).is_true()
	assert_int(field.sim.squads().size()).is_equal(3)  # the line, its reserve and the wave


func _content_field() -> FormationField:
	return FormationField.new(
		TICK,
		load("res://content/units/grem.tres"),
		8,
		load("res://content/units/kingdom_militia.tres"),
		load("res://content/units/grem_chieftain.tres")
	)


func test_a_frontal_attack_alone_does_not_break_the_line() -> void:
	var field := _content_field()
	_run(field, _has("wave_full"))

	field.send("A")
	var log := _run(field, func(log): return false, 1200)

	assert_bool(log.any(func(e): return e["type"] == "routed")).is_false()
	# Bodies, not cells (Decision 106): more of the wave wraps the line's ends than cells let
	# it, so the line loses about half (it kept about 9 of 12 before), but it holds.
	assert_int(field.kingdom_line.living().size()).is_greater_equal(4)


func test_a_flank_timed_by_the_chieftain_breaks_the_line_into_its_reserve() -> void:
	var field := _content_field()
	field.set_wait(true)
	_run(field, func(log): return field.waves["B"].built() == 9 and field.waves["A"].built() == 8)
	field.send("B")
	_run(field, _has("staged"))

	field.send("A")
	var log := _run(field, _has("routed"), 1500)
	# B pours through the ford in turn (spec 30 round 3): its line routs later, and the
	# waves reach the reserve on its hill about 100 ticks after.
	log.append_array(_run(field, func(log): return false, 150))

	var line := field.kingdom_line.id
	var reserve := field.kingdom_reserve.id
	var engaged: int = log.filter(func(e): return e["type"] == "engaged")[0]["tick"]
	var flanked: Array = log.filter(func(e): return e["type"] == "flanked")
	assert_bool(flanked.is_empty()).is_false()
	assert_int(flanked[0]["tick"] - engaged).is_less_equal(30)  # the flank lands with A
	var routs: Array = log.filter(func(e): return e["type"] == "routed" and e["squad"] == line)
	assert_bool(routs.is_empty()).is_false()
	var run_down := func(e):  # the waves go after its routers: they are fair game (Decision 111)
		return e["type"] == "engaged" and e["with"] == line and e["tick"] > routs[0]["tick"]
	assert_bool(log.any(run_down)).is_true()
	assert_bool(log.any(func(e): return e["type"] == "engaged" and e["squad"] == reserve)).is_true()


func test_route_bs_wave_crosses_the_stream_at_its_full_width() -> void:
	# Spec 30 round 3: no filing into the ford's width (part 5). Its units cross where their
	# places lie - wading the ford, or swimming the stream either side of it (everyone
	# swims, part 6) - and walk back to their places beyond.
	var field := _field(400, 200)
	_run(field, func(log): return field.waves["B"].built() == 8)
	var wave := field.send("B")
	var past := func(_log):
		return wave.living().all(func(u): return u.position.x > FormationField.STREAM.end.x + 4.0)

	_run(field, past, 900)

	(
		assert_bool(wave.living().all(func(u): return u.position.x > FormationField.STREAM.end.x))
		. is_true()
	)
	assert_int(wave.width).is_equal(8)  # it kept its shape: no narrowing


func test_the_wood_slows_route_bs_wave() -> void:
	var field := _field(400, 200)
	_run(field, func(log): return field.waves["B"].built() == 8)

	var wave := field.send("B")
	_run(field, func(log): return false, 100)

	assert_float(wave.position.x).is_less(60.0)  # open ground would be 80 cells on


func test_the_wood_hides_deep_inside_but_not_at_its_edge() -> void:
	var ground: FormationTerrain = _field().sim.terrain
	var line_front := Vector2(FormationField.LINE_AT - 10.0, 32.5)

	assert_bool(FormationSight.clear(ground, Vector2(40.5, 8.5), line_front)).is_false()
	(
		assert_bool(
			FormationSight.clear(ground, FormationField.STAGING + Vector2(0.5, 0.5), line_front)
		)
		. is_true()
	)


func test_a_captained_line_turns_to_meet_b_before_it_strikes() -> void:
	var captain := _def(60, 2)
	captain.leadership = 2
	var field := FormationField.new(TICK, _def(200, 3), 8, _def(400, 2), null, captain)
	_run(field, func(log): return field.waves["B"].built() == 8)

	field.send("B")
	var log := _run(field, _has("flanked"))

	var faced: Array = log.filter(func(e): return e["type"] == "faced")
	assert_bool(faced.is_empty()).is_false()
	assert_int(faced[0]["squad"]).is_equal(field.kingdom_line.id)
	assert_int(faced[0]["facing"]).is_equal(SquadFrame.NORTH)
