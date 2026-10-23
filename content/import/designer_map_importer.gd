class_name DesignerMapImporter
## A designer export (a saved designer map, specs 16 and 19) as a MapDef, and a designer
## structure plan (spec 24) as a StructurePlanDef, for the tests that build maps and plans as
## the designer draws them. The reading and the checks are the Rust crate's
## (native/rust/src/designer/, spec 39 step 2c): the designer's save runs the same, so nothing
## here can drift from what the game loads.


class DesignerMapImportResult:
	extends RefCounted
	var map_def: MapDef = MapDef.new()
	var errors: PackedStringArray = PackedStringArray()
	var warnings: PackedStringArray = PackedStringArray()


## The map the designer's `export_data` describes, its terrain and features checked against
## the shared terrain library and its rules checked.
static func import_map(export_data: Dictionary) -> DesignerMapImportResult:
	var result := DesignerMapImportResult.new()
	var store := CrateContent.loaded_store()
	if store == null:
		result.errors.append("the native core isn't built: run bash native/build.sh")
		return result
	CrateContent.terrain_library()
	var read: Dictionary = store.map_from_export(export_data)
	result.map_def = read["map"]
	result.errors = PackedStringArray(read["errors"])
	result.warnings = PackedStringArray(read["warnings"])
	return result


## The plan the designer drew (solid cells, faces, dug cells, fills and loads).
static func plan(drawn: Dictionary) -> StructurePlanDef:
	var store := CrateContent.loaded_store()
	return store.plan_from_export(drawn) if store else StructurePlanDef.new()
