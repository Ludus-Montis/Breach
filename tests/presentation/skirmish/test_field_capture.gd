extends GdUnitTestSuite
## FieldCapture, spec 44 step 2c (Decision 169): the 2D field started with
## `-- --capture=<name>` plays that scenario hands-free and writes capture.json. A short
## scenario (field_break_off, 291 ticks) is played to its end without quitting the test.

const SCENE := "res://presentation/skirmish/formation_2d/formation_field.tscn"
const SHORT := "field_break_off"
const OUT := "user://test_field_capture/capture.json"


func _captured(name: String) -> FormationFieldScene:
	var scene: FormationFieldScene = auto_free(load(SCENE).instantiate())
	scene.capture = FieldCapture.new(name, OUT)
	scene.capture.quits = false
	add_child(scene)
	scene.clock().pause()  # the test owes the ticks itself
	return scene


## The scenario played to its end, a frame at a time; capture.json's text.
func _played_to_end(name: String) -> String:
	DirAccess.remove_absolute(ProjectSettings.globalize_path(OUT))
	var scene := _captured(name)
	for frame in 1000:
		scene.capture.frame(scene, 1)
		if scene.capture.written:
			break
	return FileAccess.get_file_as_string(OUT)


func test_the_capture_is_read_from_the_command_line() -> void:
	var asked := FieldCapture.requested(
		PackedStringArray(["--write-movie", "/tmp/out/x.avi", "--", "--capture=field_a"])
	)
	assert_str(asked.name).is_equal("field_a")
	assert_str(asked.out).is_equal("/tmp/out/capture.json")
	var placed := FieldCapture.requested(
		PackedStringArray(["--capture=b", "--capture-out=/o.json"])
	)
	assert_str(placed.out).is_equal("/o.json")
	assert_object(FieldCapture.requested(PackedStringArray(["--path", "."]))).is_null()


func test_a_scenario_plays_hands_free_and_writes_its_capture_once_ended() -> void:
	var made: Dictionary = JSON.parse_string(_played_to_end(SHORT))

	assert_str(made["scenario"]).is_equal(SHORT)
	assert_str(made["map"]).is_equal("field")
	var moments: Dictionary = made["moments"]
	var ticks := int(made["ticks"])
	assert_int(int(moments["first"])).is_equal(0)
	assert_int(int(moments["contact"])).is_greater(0)
	assert_int(int(moments["midpoint"])).is_greater_equal(int(moments["contact"]))
	assert_int(int(moments["last"])).is_equal(ticks - 1)
	assert_int(ticks).is_less_equal(900)  # by 90 s of play
	assert_that(made["behaviours"]["engaging"]).is_not_null()
	assert_that(made["behaviours"]["flanking"]).is_null()
	assert_array(made["missing"]).is_empty()


func test_the_same_scenario_twice_writes_the_same_capture() -> void:
	assert_str(_played_to_end(SHORT)).is_equal(_played_to_end(SHORT))


func test_a_name_not_on_the_list_is_refused_naming_it() -> void:
	var scene: FormationFieldScene = auto_free(load(SCENE).instantiate())
	add_child(scene)

	assert_str(scene.restart(false, 0, [], "field_nowhere")).contains("field_nowhere")
	var capture := FieldCapture.new("field_nowhere", OUT)
	capture.quits = false
	assert_bool(capture.start(scene)).is_false()


func test_without_the_argument_the_field_plays_as_before() -> void:
	var scene: FormationFieldScene = auto_free(load(SCENE).instantiate())
	add_child(scene)

	assert_object(scene.capture).is_null()
	assert_bool(scene.field().ended()).is_false()
	assert_str(scene.field().capture("abc")).is_empty()
