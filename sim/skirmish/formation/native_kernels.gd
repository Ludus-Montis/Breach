class_name NativeKernels
extends RefCounted
## The switch for the simulation's native core (Decision 129, native/README.md): whether
## the hot passes - body parting (UnitBodies) and the scrum's slot search (ScrumSeek) - run
## in GDScript ("gdscript", the default and the reference) or in Rust ("rust": a BodyField,
## the bodies' state owned natively). Read from the BREACH_NATIVE environment variable,
## else the project setting breach/native/engine; tests and the bench set it with use(). The
## library itself is always loaded, for the content (CrateContent, Decision 153); this picks
## only the kernels (NativeLibrary loads it). BREACH_NATIVE_THREADS=1 lets the Rust core
## part bodies on its thread pool (applied in a fixed order all the same). Both engines give
## bit-identical outcomes (tools/formation_digest.gd).

const GDSCRIPT := "gdscript"
const RUST := "rust"
const FIELD_CLASS := "BodyField"
const UnitStoreGd = preload("res://sim/skirmish/formation/unit_store_gd.gd")
const UnitStoreNative = preload("res://sim/skirmish/formation/unit_store_native.gd")

## Whether the native core may use threads (Rayon's pool).
static var threaded := false
static var _engine := ""


## The engine chosen: use()'s, else BREACH_NATIVE's, else the project setting's.
static func engine() -> String:
	if _engine == "":
		_engine = OS.get_environment("BREACH_NATIVE")
		if _engine == "":
			_engine = str(ProjectSettings.get_setting("breach/native/engine", GDSCRIPT))
		threaded = OS.get_environment("BREACH_NATIVE_THREADS") == "1"
	return _engine


## Picks the engine from now on ("gdscript" or "rust").
static func use(engine_name: String) -> void:
	_engine = engine_name


## True if the engine runs here: GDScript always, Rust once its library is built.
static func available(engine_name: String) -> bool:
	if engine_name == GDSCRIPT:
		return true
	if engine_name != RUST:
		return false
	return NativeLibrary.load_for(FIELD_CLASS)


## A new field for a battle's bodies (BodyField) under the chosen engine, or null to run
## the GDScript passes.
static func body_field() -> Object:
	var engine_name := engine()
	if engine_name == GDSCRIPT or not available(engine_name):
		return null
	return ClassDB.instantiate(FIELD_CLASS)


## The battle's unit store (spec 35): on its native field, or in GDScript if it has none.
static func unit_store(field: Object) -> UnitStore:
	return UnitStoreGd.new() if field == null else UnitStoreNative.new(field)
