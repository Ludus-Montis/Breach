class_name DesignerMapImporter
## A designer export (a saved designer map, specs 16 and 19) as a MapDef, and a designer
## structure plan (spec 24) as a StructurePlanDef, for the tests that build maps and plans as
## the designer draws them. The reading is the Rust crate's (native/rust/src/designer/, spec
## 39 step 2c): the designer's server runs the same rules when it saves a map to
## content/maps/<name>.json, so nothing here can drift from what the game loads.

const STORE_CLASS := "ContentStore"
const ROOT := "res://content"


class DesignerMapImportResult:
	extends RefCounted
	var map_def: MapDef = MapDef.new()
	var errors: PackedStringArray = PackedStringArray()
	var warnings: PackedStringArray = PackedStringArray()


## The map the designer's `export_data` describes, its terrain and features checked against
## the shared terrain library.
static func import_map(export_data: Dictionary) -> DesignerMapImportResult:
	var result := DesignerMapImportResult.new()
	if not NativeLibrary.load_for(STORE_CLASS):
		result.errors.append("the native core isn't built: run bash native/build.sh")
		return result
	var store: Object = ClassDB.instantiate(STORE_CLASS)
	var read: Dictionary = store.map_from_export(export_data, ProjectSettings.globalize_path(ROOT))
	result.map_def = MapFill.build(read["map"])
	result.errors = PackedStringArray(read["errors"])
	result.warnings = PackedStringArray(read["warnings"])
	return result


## The plan the designer drew (solid cells, faces, dug cells, fills and loads).
static func plan(drawn: Dictionary) -> StructurePlanDef:
	if not NativeLibrary.load_for(STORE_CLASS):
		push_error("DesignerMapImporter: the native core isn't built: run bash native/build.sh")
		return StructurePlanDef.new()
	var read: Dictionary = ClassDB.instantiate(STORE_CLASS).plan_from_export(drawn)
	for error: String in read["errors"]:
		push_error("DesignerMapImporter: %s" % error)
	return ContentFill.fill(StructurePlanDef.new(), read["plan"])
