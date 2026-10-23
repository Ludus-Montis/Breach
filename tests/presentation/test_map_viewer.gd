extends GdUnitTestSuite
## MapViewer (presentation/map_viewer.tscn), per specs/18-map-viewer.md: argument
## parsing and map discovery, plus a smoke test that the scene instantiates and loads
## a map. Interaction (pan, zoom, pick, click) is verified manually.

const MapViewer = preload("res://presentation/map_viewer.gd")
const VIEWER_SCENE := "res://presentation/map_viewer.tscn"


func test_map_argument_is_read_from_user_args() -> void:
	var args := PackedStringArray(["--other", "--map=demo_map"])

	assert_str(MapViewer.map_name_from_args(args)).is_equal("demo_map")
	assert_str(MapViewer.map_name_from_args(PackedStringArray())).is_equal("")


func test_lists_the_map_files_in_name_order() -> void:
	var names := MapViewer.list_map_names("res://content/maps")

	assert_array(Array(names)).contains(["demo_map", "p_f_F_c"])
	var sorted := Array(names)
	sorted.sort()
	assert_array(Array(names)).is_equal(sorted)


func test_files_that_arent_maps_are_skipped() -> void:
	var names := MapViewer.list_map_names("res://assets/placeholder/map")

	assert_array(Array(names)).is_empty()


func test_scene_instantiates_and_shows_the_requested_map() -> void:
	var viewer: MapViewer = auto_free(load(VIEWER_SCENE).instantiate())
	viewer.initial_map_name = "demo_map"

	add_child(viewer)

	assert_object(viewer.map_view().map).is_not_null()
	assert_int(viewer.map_view().model.markers.size()).is_greater(0)
