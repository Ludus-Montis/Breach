extends GdUnitTestSuite
## The shared load-path cases (tests/fixtures/structure/load_cases.json), per spec 24
## round 2: the same cases run against LoadPaths here and against the designer's
## tools/designer/load_paths.js, so the two implementations stay one rule.

const LoadPaths = preload("res://sim/structure/load_paths.gd")
const DesignerMapImporter = preload("res://content/import/designer_map_importer.gd")
const MaterialDef = preload("res://content/definitions/material_def.gd")
const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")

const CASES := "res://tests/fixtures/structure/load_cases.json"


func _fixture() -> Dictionary:
	return JSON.parse_string(FileAccess.get_file_as_string(CASES))


func _library(entries: Array) -> TerrainLibraryDef:
	var library := TerrainLibraryDef.new()
	for entry in entries:
		var material := MaterialDef.new()
		material.id = entry["id"]
		material.weight = int(entry["weight"])
		material.strength = int(entry["strength"])
		material.span = int(entry["span"])
		if entry.has("flows"):
			material.traits = {"flows": int(entry["flows"])}
		library.materials.append(material)
	return library


func test_every_shared_case_holds() -> void:
	var fixture := _fixture()
	var library := _library(fixture["materials"])
	for case in fixture["cases"]:
		var plan := DesignerMapImporter.plan(case["plan"])
		var result := LoadPaths.solve(plan, library, _bearing(case["bearing"]))
		assert_array(result["failed"]).override_failure_message(case["name"]).is_equal(
			case["failed"]
		)
		for key in case.get("loads", {}):
			assert_int(result["loads"][key]).override_failure_message(case["name"]).is_equal(
				int(case["loads"][key])
			)
		if case.has("settled"):
			assert_array(LoadPaths.settle(plan, library, _bearing(case["bearing"]))).is_equal(
				case["settled"]
			)


## A case's bearing: one number for every column, or {default, columns: {"x,y": n}}.
func _bearing(spec) -> Variant:
	if not spec is Dictionary:
		return int(spec)
	var columns := {}
	for at in spec.get("columns", {}):
		var parts: PackedStringArray = at.split(",")
		columns[Vector2i(int(parts[0]), int(parts[1]))] = int(spec["columns"][at])
	return {"default": int(spec.get("default", 0)), "columns": columns}
