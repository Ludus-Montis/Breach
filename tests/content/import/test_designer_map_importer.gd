extends GdUnitTestSuite
## DesignerMapImporter: the designer's exports and plans as definitions, read by the Rust
## crate (spec 39 step 2c). The reading's own scenarios (specs 16, 19, 24, 26) are cargo
## tests (native/rust/src/designer/tests.rs); these check what reaches GDScript.

const DesignerMapImporter = preload("res://content/import/designer_map_importer.gd")

const DEMO_PATH := "res://tests/fixtures/designer/demo_map.designer.json"
const LAYOUT_PATH := "res://tests/fixtures/designer/layout_map.designer.json"


func _export(path: String) -> Dictionary:
	return JSON.parse_string(FileAccess.get_file_as_string(path))


func test_the_demo_export_is_a_valid_map_with_its_lane_and_off_lane_nodes() -> void:
	var result := DesignerMapImporter.import_map(_export(DEMO_PATH))

	assert_array(result.errors).is_empty()
	(
		assert_array(result.map_def.lanes[0].nodes.map(func(n): return n.id))
		. is_equal(["p", "f", "F", "c"])
	)
	assert_array(result.map_def.off_lane_nodes.map(func(n): return n.id)).is_equal(["s2", "w1"])


func test_a_layout_export_is_a_valid_map_on_the_shared_library() -> void:
	var result := DesignerMapImporter.import_map(_export(LAYOUT_PATH))

	assert_array(result.errors).is_empty()
	assert_object(result.map_def.layout.terrain_library).is_same(CrateContent.terrain_library())
	assert_object(result.map_def.layout.tile_at(Vector2i(7, 3)).bridge).is_instanceof(BridgeDef)


func test_errors_and_warnings_reach_gdscript() -> void:
	var export_data := _export(DEMO_PATH)
	export_data["format_version"] = 2
	assert_bool(Array(DesignerMapImporter.import_map(export_data).errors).is_empty()).is_false()
	var warnings := DesignerMapImporter.import_map(_export(DEMO_PATH)).warnings
	assert_bool(Array(warnings).any(func(w): return w.contains("structures"))).is_true()
