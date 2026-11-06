extends GdUnitTestSuite
## FormationFieldScene smoke tests, per Decisions 86 and 87 and spec 27: the 2D field
## builds, its waves fill and can be sent, and squads stand in 2D on their routes. The field
## is the crate's (`FieldState`, spec 39 step 9d-i), played on a worker (step 10d-ii) and read
## by the scene and its HUD from its newest snapshot. How it plays - marching, wheeling,
## meeting head-on - is checked by hand.

const FormationFieldScene = preload(
	"res://presentation/skirmish/formation_2d/formation_field_scene.gd"
)
const FormationFieldHud = preload("res://presentation/skirmish/formation_2d/formation_field_hud.gd")
const SCENE := "res://presentation/skirmish/formation_2d/formation_field.tscn"


func _scene() -> FormationFieldScene:
	var scene: FormationFieldScene = auto_free(load(SCENE).instantiate())
	add_child(scene)
	scene.clock().pause()  # the test drives ticks itself
	return scene


func _hud(scene: FormationFieldScene) -> FormationFieldHud:
	return scene.get_children().filter(func(c): return c is FormationFieldHud)[0]


func test_the_scene_builds_the_field_with_the_kingdom_line() -> void:
	var scene := _scene()

	assert_object(scene.field()).is_not_null()
	assert_array(scene.field().routes().keys()).contains_exactly(["A", "B", "C"])
	assert_int(scene.field().squads().size()).is_equal(2)  # the line and its reserve
	assert_array(Array(scene.field().kingdom())).has_size(2)


func test_a_sent_wave_marches_down_its_route_in_2d() -> void:
	var scene := _scene()
	scene.run_ticks(100)  # four grem builders fill an 8-wide wave

	scene.act("send A")
	scene.run_ticks(20)

	var squads: Array = scene.field().squads()
	assert_int(squads.size()).is_equal(3)
	var wave: Dictionary = squads[2]
	assert_str(wave["faction"]).is_equal("player")
	assert_float(wave["position"].y).is_equal_approx(32.0, 0.01)
	assert_float(wave["position"].x).is_greater(0.0)
	var at: PackedVector2Array = scene.field().positions()
	for unit in wave["units"]:
		assert_float(at[unit["slot"]].y).is_between(27.0, 37.0)


func test_waves_can_be_sent_together_and_b_can_wait() -> void:
	var scene := _scene()
	scene.act("wait on")
	scene.run_ticks(200)

	scene.act("send A+B")
	scene.run_ticks(5)

	assert_int(scene.field().squads().size()).is_equal(4)
	assert_object(scene.field().staging_point()).is_equal(Vector2(56, 21))


func test_the_line_can_be_given_a_captain() -> void:
	var scene := _scene()

	scene.restart(true, 7)

	var line: Dictionary = scene.field().squads()[0]
	assert_bool(line["units"].any(func(u): return u["leadership"] > 0)).is_true()


func test_reset_starts_a_fresh_field_under_a_seed() -> void:
	var scene := _scene()
	scene.run_ticks(50)
	var before = scene.field()

	scene.restart(false, 42)

	assert_object(scene.field()).is_not_same(before)
	assert_int(scene.field().battle_seed()).is_equal(42)
	assert_int(scene.field().tick_number()).is_equal(0)


func test_a_sent_wave_can_be_ordered_to_retreat() -> void:
	var scene := _scene()
	scene.run_ticks(100)
	scene.act("send A")
	scene.run_ticks(5)

	scene.act("retreat A")
	var events := scene.run_ticks(1)

	var applied := events.filter(func(e): return e["type"] == "order_applied")
	assert_array(applied).is_not_empty()
	assert_str(applied[0]["order"]).is_equal("RETREAT")


func test_actions_are_logged_with_their_ticks_after_the_seed() -> void:
	var scene := _scene()
	scene.restart(false, 42)
	scene.run_ticks(5)

	scene.act("send A")
	scene.run_ticks(0)  # the order is taken at the next boundary between ticks

	var head := "record 2 seed 42 captain off build %s" % SimBuild.id()
	assert_str(scene.action_log()).is_equal(head + "\n5 player send A")


func test_a_pasted_log_replays_live_as_the_headless_replay_does() -> void:
	var log := (
		"record 2 seed 446157 captain off build %s\n0 player wait waves on\n40 player send A+B"
		% SimBuild.id()
	)
	var scene := _scene()
	var read: Dictionary = NativeLibrary.made("FieldState").read_record(log)
	scene.restart(read["captained"], read["seed"], read["commands"])

	scene.run_ticks(200)

	var headless: Object = _replayed(read, 200)
	assert_str(scene.action_log()).is_equal(log)
	assert_int(scene.field().tick_number()).is_equal(headless.tick_number())
	assert_str(str(_fronts(scene.field()))).is_equal(str(_fronts(headless)))


## `read`'s field played headless: each command given at its tick, `ticks` ticks in all.
func _replayed(read: Dictionary, ticks: int) -> Object:
	var field: Object = NativeLibrary.made("FieldState")
	field.open(CrateContent.store(), read["seed"], read["captained"], SimBuild.id())
	var queued: Array = read["commands"].duplicate()
	for _i in range(ticks):
		while not queued.is_empty() and queued[0]["tick"] <= field.tick_number():
			field.give(queued.pop_front())
		field.step()
	return field


func _fronts(field: Object) -> Array:
	var rows := []
	for squad in field.squads():
		var living: int = squad["units"].filter(func(u): return u["alive"]).size()
		rows.append([squad["state"], squad["position"], living])
	return rows


func test_replay_plays_the_pasted_log_not_this_runs() -> void:
	var scene := _scene()
	scene.run_ticks(3)
	scene.act("send A")
	var hud := _hud(scene)
	hud._replay_view.text = "seed 492625 captain off\n22 via_c on\n129 send A+B"
	hud.show_status(scene.field(), true, 1)  # a frame passes: the pasted log is kept

	hud.replay()
	scene.run_ticks(130)

	var recorded := "record 2 seed 492625 captain off build %s" % SimBuild.id()
	recorded += "\n22 player route A C\n129 player send A+B"
	assert_str(scene.action_log()).is_equal(recorded)  # an old log, recorded afresh


func test_the_hud_counts_the_waves_places_and_the_line() -> void:
	var scene := _scene()
	scene.restart(true, 7)
	var hud := _hud(scene)
	scene.run_ticks(1)

	hud.show_status(scene.field(), true, 7)

	assert_str(hud._status.text).contains("Waves: A 0/8, B 0/8 + leader 0/1")
	assert_str(hud._status.text).contains("Line: 12 (holding)")


func test_a_pursuing_line_shows_how_far_it_has_gone_of_its_leash() -> void:
	var scene := _scene()
	scene.restart(true, 606531)  # a captained line: discipline 50, a 64-cell leash
	scene.act("pursues on")
	var hud := _hud(scene)
	var sent := false
	for _i in range(3000):
		if not sent and scene.field().waves()["A"][0] == 8:
			scene.act("send A")
			sent = true
		scene.run_ticks(1)
		if sent and scene.field().squads()[-1]["state"] == "FIGHTING":
			break
	scene.run_ticks(30)
	scene.act("retreat A")
	scene.run_ticks(40)

	hud.show_status(scene.field(), true, 606531)
	assert_str(hud._status.text).contains("pursuing ")
	assert_str(hud._status.text).contains("/64 cells")


func test_a_frame_owes_its_clocks_ticks_and_draws_what_the_worker_has_played() -> void:
	var scene := _scene()
	scene.clock().resume()

	scene._process(0.1)  # one tick due: owed to the worker, not waited for
	var began := Time.get_ticks_msec()
	while scene.field().tick_number() < 1 and Time.get_ticks_msec() - began < 10000:
		OS.delay_msec(5)
		scene._process(0.0)

	assert_int(scene.field().tick_number()).is_greater_equal(1)
	assert_bool(scene.field().caught_up()).is_true()


func test_the_hud_says_the_simulation_is_behind_when_time_is_dropped() -> void:
	var scene := _scene()
	scene.clock().resume()

	scene._process(1.0)  # ten ticks due: the worker is owed at most two

	assert_str(_hud(scene)._status.text).contains("SIMULATION BEHIND")
