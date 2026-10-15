extends GdUnitTestSuite
## FormationFieldScene smoke tests, per Decisions 86 and 87 and spec 27: the 2D field
## builds, its waves fill and can be sent, and squads stand in 2D on their routes. How it
## plays - marching, wheeling, meeting head-on - is checked by hand.

const FormationFieldScene = preload(
	"res://presentation/skirmish/formation_2d/formation_field_scene.gd"
)
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationFieldHud = preload("res://presentation/skirmish/formation_2d/formation_field_hud.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SCENE := "res://presentation/skirmish/formation_2d/formation_field.tscn"


func _scene() -> FormationFieldScene:
	var scene: FormationFieldScene = auto_free(load(SCENE).instantiate())
	add_child(scene)
	scene.clock().pause()  # the test drives ticks itself
	return scene


func test_the_scene_builds_the_field_with_the_kingdom_line() -> void:
	var scene := _scene()

	assert_object(scene.field()).is_not_null()
	assert_array(scene.field().routes.keys()).contains_exactly(["A", "B", "C"])
	assert_int(scene.field().sim.squads().size()).is_equal(2)  # the line and its reserve


func test_a_sent_wave_marches_down_its_route_in_2d() -> void:
	var scene := _scene()
	scene.run_ticks(100)  # four grem builders fill an 8-wide wave

	scene.act("send A")
	scene.run_ticks(20)

	var squads := scene.field().sim.squads()
	assert_int(squads.size()).is_equal(3)
	var wave = squads[2]
	assert_float(wave.position.y).is_equal_approx(32.0, 0.01)
	assert_float(wave.position.x).is_greater(0.0)
	for unit in wave.living():
		var at: Vector2 = wave.store.columns.value(unit.slot, UnitColumns.POSITION)
		assert_float(at.y).is_between(27.0, 37.0)


func test_waves_can_be_sent_together_and_b_can_wait() -> void:
	var scene := _scene()
	scene.field().set_wait(true)
	scene.run_ticks(200)

	var sent: Array = scene.field().send_together(["A", "B"])
	scene.run_ticks(5)

	assert_int(sent.size()).is_equal(2)
	assert_bool(scene.field().waves["B"].staging.is_empty()).is_false()


func test_the_line_can_be_given_a_captain() -> void:
	var scene := _scene()

	scene.restart(true, 7)

	var line = scene.field().kingdom_line
	assert_bool(line.living().any(func(u): return u.leadership > 0)).is_true()


func test_reset_starts_a_fresh_field_under_a_seed() -> void:
	var scene := _scene()
	scene.run_ticks(50)
	var before = scene.field()

	scene.restart(false, 42)

	assert_object(scene.field()).is_not_same(before)
	assert_int(scene.field().sim.fight_seed).is_equal(42)
	assert_int(scene.field().sim.tick_number()).is_equal(0)


func test_a_sent_wave_can_be_ordered_to_retreat() -> void:
	var scene := _scene()
	scene.run_ticks(100)
	scene.act("send A")
	scene.run_ticks(5)

	scene.act("retreat A")
	scene.run_ticks(1)

	var wave = scene.field().sim.squads()[2]
	assert_int(wave.order).is_equal(2)  # SkirmishUnit.Order.RETREAT


func test_actions_are_logged_with_their_ticks_after_the_seed() -> void:
	var scene := _scene()
	scene.restart(false, 42)
	scene.run_ticks(5)

	scene.act("send A")

	var head := "record 2 seed 42 captain off build %s" % SimBuild.id()
	assert_str(scene.action_log()).is_equal(head + "\n5 player send A")


func test_a_pasted_log_replays_live_as_the_headless_replay_does() -> void:
	var log := (
		"record 2 seed 446157 captain off build %s\n0 player wait waves on\n40 player send A+B"
		% SimBuild.id()
	)
	var scene := _scene()
	var read := FormationFieldActions.parse(log)
	scene.restart(read["captained"], read["seed"], read["commands"])

	scene.run_ticks(200)

	var headless := FormationFieldActions.replay(log, 200 - 40)
	assert_str(scene.action_log()).is_equal(log)
	assert_int(scene.field().sim.tick_number()).is_equal(headless.sim.tick_number())
	assert_str(str(_fronts(scene.field()))).is_equal(str(_fronts(headless)))


func _fronts(field: FormationField) -> Array:
	var rows := []
	for squad in field.sim.squads():
		rows.append([squad.state, squad.position, squad.living().size()])
	return rows


func test_replay_plays_the_pasted_log_not_this_runs() -> void:
	var scene := _scene()
	scene.run_ticks(3)
	scene.act("send A")
	var hud: FormationFieldHud = scene.get_children().filter(func(c): return c is FormationFieldHud)[0]
	hud._replay_view.text = "seed 492625 captain off\n22 via_c on\n129 send A+B"
	hud.show_status(scene.field(), true, 1)  # a frame passes: the pasted log is kept

	hud.replay()
	scene.run_ticks(130)

	var recorded := "record 2 seed 492625 captain off build %s" % SimBuild.id()
	recorded += "\n22 player route A C\n129 player send A+B"
	assert_str(scene.action_log()).is_equal(recorded)  # an old log, recorded afresh


func test_a_pursuing_line_shows_how_far_it_has_gone_of_its_leash() -> void:
	var scene := _scene()
	scene.restart(true, 606531)  # a captained line: discipline 50, a 64-cell leash
	scene.act("pursues on")
	var hud: FormationFieldHud = scene.get_children().filter(func(c): return c is FormationFieldHud)[0]
	var wave = null
	for _i in range(3000):
		if wave == null and scene.field().waves["A"].built() == 8:
			scene.act("send A")
			wave = scene.field().sim.squads()[-1]
		scene.run_ticks(1)
		if wave != null and wave.state == SkirmishSquad.State.FIGHTING:
			break
	scene.run_ticks(30)
	scene.act("retreat A")
	scene.run_ticks(40)

	hud.show_status(scene.field(), true, 606531)
	assert_str(hud._status.text).contains("pursuing ")
	assert_str(hud._status.text).contains("/64 cells")
