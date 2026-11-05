extends GdUnitTestSuite
## FormationSkirmishScene smoke tests, per specs/22-formation-feel-test.md: two lanes from
## the map, a shared slot pool, and the wave-full pause (Decision 39). The battle is the
## crate's (`BattleState`, spec 39 step 9d-ii). Playing it - width, flanking, step-up,
## splitting the pool - is checked by hand.

const FormationSkirmishScene = preload(
	"res://presentation/skirmish/formation/formation_skirmish_scene.gd"
)
const SkirmishSquadLayer = preload("res://presentation/skirmish/formation/skirmish_squad_layer.gd")
const SCENE := "res://presentation/skirmish/formation/formation_skirmish.tscn"
const SCRIPTS := [
	"res://presentation/skirmish/formation/formation_skirmish_scene.gd",
	"res://presentation/skirmish/formation/formation_skirmish_hud.gd",
	"res://presentation/skirmish/formation/formation_skirmish_readout.gd",
	"res://presentation/skirmish/formation/skirmish_squad_layer.gd",
	"res://presentation/skirmish/formation/wave_painter.gd",
]


func _scene() -> FormationSkirmishScene:
	var scene: FormationSkirmishScene = auto_free(load(SCENE).instantiate())
	add_child(scene)
	return scene


func test_the_scene_builds_a_simulation_per_lane_from_the_map() -> void:
	var scene := _scene()

	assert_array(scene.lane_keys()).contains_exactly(["c", "k"])
	assert_float(scene.battle().route_length("c")).is_greater(9.0)
	assert_float(scene.battle().route_length("k")).is_greater(9.0)


func test_the_pool_counts_what_each_lane_has_painted() -> void:
	var scene := _scene()

	assert_int(scene.battle().pool()["assigned"]["c"]).is_equal(5)
	assert_int(scene.battle().pool()["assigned"]["k"]).is_equal(3)
	assert_int(scene.battle().pool()["free"]).is_equal(0)


func test_erasing_in_one_lane_frees_slots_for_the_other_at_once() -> void:
	var scene := _scene()
	scene._choose_brush(0)  # grem
	scene._paint_cell("k", Vector2i(0, 0), true)
	scene._paint_cell("k", Vector2i(0, 1), true)

	scene._paint_cell("c", Vector2i(1, 0))
	scene._paint_cell("c", Vector2i(1, 1))

	assert_int(scene.battle().pool()["assigned"]["c"]).is_equal(7)
	assert_int(scene.battle().pool()["assigned"]["k"]).is_equal(1)
	assert_int(scene.battle().pool()["free"]).is_equal(0)


func test_a_saved_preset_applies_to_another_lane_fitted_to_its_slots() -> void:
	var scene := _scene()
	scene.presets_path = ""  # never touch the presets saved on this machine
	scene._presets = []
	scene._save_preset("c")  # lane c's 5-wide line

	scene._apply_preset("k", 0)  # lane k holds only its own 3 slots (none free)

	assert_int(scene._presets.size()).is_equal(1)
	assert_int(scene.battle().pool()["assigned"]["k"]).is_equal(3)
	assert_int(scene.battle().pool()["free"]).is_equal(0)


func test_a_full_wave_pauses_and_sending_it_resumes() -> void:
	var scene := _scene()
	scene.pause_on_wave_full = true
	scene.battle().set_auto_departure("c", false)

	scene.handle_events("c", [{"type": "wave_full", "tick": 1, "faction": "player", "built": 5}])
	assert_bool(scene.clock().is_paused()).is_true()
	scene.send_wave("c")

	assert_bool(scene.clock().is_paused()).is_false()


func test_lanes_start_departing_on_their_own_and_sharing_round_robin() -> void:
	var scene := _scene()

	assert_bool(scene.battle().wave("c")["auto"]).is_true()
	assert_bool(scene.battle().sharing()["round_robin"]).is_true()
	assert_int(scene.battle().combat_width("c")).is_equal(8)


func test_auto_merge_marks_the_waves_a_lane_sends() -> void:
	var scene := _scene()
	scene._hud.auto_merge_toggled.emit("c", true)

	assert_bool(scene.battle().wave("c")["merges"]).is_true()
	assert_bool(scene.battle().wave("k")["merges"]).is_false()


func test_a_lane_that_departs_on_its_own_never_pauses() -> void:
	var scene := _scene()
	scene.pause_on_wave_full = true

	scene.handle_events("c", [{"type": "wave_full", "tick": 1, "faction": "player", "built": 5}])

	assert_bool(scene.clock().is_paused()).is_false()


func test_notify_only_keeps_the_game_running() -> void:
	var scene := _scene()
	scene.pause_on_wave_full = false

	scene.handle_events("k", [{"type": "wave_full", "tick": 1, "faction": "player", "built": 3}])

	assert_bool(scene.clock().is_paused()).is_false()


func test_the_squad_layer_constructs() -> void:
	var layer: SkirmishSquadLayer = auto_free(SkirmishSquadLayer.new())

	assert_object(layer).is_not_null()


func test_units_in_different_lanes_with_the_same_id_stay_apart() -> void:
	var scene := _scene()
	scene._hud.spawn_kingdom_pressed.emit("c")
	scene._hud.spawn_kingdom_pressed.emit("k")
	var layer: SkirmishSquadLayer = scene.get_node("Squads")
	layer.snapshot()
	var in_c: Dictionary = layer.squads("c")[0]["units"][0]
	var in_k: Dictionary = layer.squads("k")[0]["units"][0]

	assert_int(in_c["id"]).is_equal(in_k["id"])  # ids are per lane
	assert_float(layer._distance("c", in_c)).is_equal_approx(scene.battle().route_length("c"), 0.01)
	assert_float(layer._distance("k", in_k)).is_equal_approx(scene.battle().route_length("k"), 0.01)


# --- Decision 42/43: painted templates, applied at once ---------------------------


func test_painting_a_brute_changes_the_lanes_template_at_once() -> void:
	var scene := _scene()
	for column in range(3):
		scene._paint_cell("k", Vector2i(0, column), true)  # free lane k's 3 slots
	scene._choose_brush(1)  # brute

	scene._paint_cell("c", Vector2i(0, 1))

	var places: Array = scene.battle().wave("c")["places"]
	assert_bool(places.any(func(p): return p[2] == 2 and p[3] == 2 and p[0] == 0)).is_true()


# --- Decision 45: the domain's builders and how units are shared out ---------------


func test_the_builder_buttons_add_and_remove_builders() -> void:
	var scene := _scene()

	scene._set_builders(0, 1)
	scene._set_builders(1, -5)

	assert_int(scene.battle().builder_count(0)).is_equal(3)
	assert_int(scene.battle().builder_count(1)).is_equal(0)


func test_the_sharing_picker_sets_lane_priority_or_round_robin() -> void:
	var scene := _scene()

	scene._choose_distribution(1)  # "k first"
	assert_str(scene.battle().sharing()["first"]).is_equal("k")
	assert_bool(scene.battle().sharing()["round_robin"]).is_false()
	scene._choose_distribution(2)  # "Round robin"

	assert_bool(scene.battle().sharing()["round_robin"]).is_true()


# --- Decision 48: small units, so the camera zooms and pans ------------------------


func test_the_camera_zooms_toward_a_point_within_limits() -> void:
	var scene := _scene()
	var camera = scene.get_node("Camera")

	camera.zoom_by(1000.0, Vector2.ZERO)
	var closest: float = camera.zoom.x
	camera.zoom_by(0.0001, Vector2.ZERO)

	assert_float(closest).is_equal_approx(camera.MAX_ZOOM, 0.0001)
	assert_float(camera.zoom.x).is_equal_approx(camera.MIN_ZOOM, 0.0001)


func test_both_lanes_are_eight_wide() -> void:
	var scene := _scene()

	assert_int(scene.battle().combat_width("c")).is_equal(8)
	assert_int(scene.battle().combat_width("k")).is_equal(8)


# --- spec 39 step 9d-ii: the battle is the crate's -----------------------------------


func test_a_sent_wave_marches_down_its_lane() -> void:
	var scene := _scene()
	scene.battle().set_auto_departure("c", false)
	for i in range(60):  # two grem builders, 2 s a grem, sharing in turn
		scene._tick()
	scene.send_wave("c")
	scene._tick()
	var first: float = scene.get_node("Squads").squads("c")[0]["units"][0]["distance"]
	for i in range(10):
		scene._tick()

	var squads: Array = scene.get_node("Squads").squads("c")
	assert_str(squads[0]["faction"]).is_equal("player")
	assert_float(squads[0]["units"][0]["distance"]).is_greater(first)


func test_an_order_reaches_the_selected_squad() -> void:
	var scene := _scene()
	scene._hud.spawn_kingdom_pressed.emit("k")
	var id: int = scene.battle().squads("k")[0]["id"]
	scene.get_node("Squads").selected = ["k", id]

	scene._order(1)  # hold
	scene._tick()

	assert_str(scene.battle().squads("k")[0]["order"]).is_equal("HOLD")


func test_the_scenes_scripts_load_nothing_under_sim() -> void:
	for path in SCRIPTS:
		assert_str(load(path).source_code).not_contains("res://sim/")
