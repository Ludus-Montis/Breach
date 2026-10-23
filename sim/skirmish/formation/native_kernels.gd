class_name NativeKernels
extends RefCounted
## The switch for the simulation's last GDScript twin (Decision 129, native/README.md):
## whether body parting (UnitBodies) runs in GDScript ("gdscript", the default and the
## reference) or in Rust ("rust"). Read from the BREACH_NATIVE environment variable, else
## the project setting breach/native/engine; tests and the bench set it with use(). Every
## battle has its bodies' state on a native BodyField and its unit store there, under
## either engine (spec 39 step 4a): the scrum's passes run on it, and their twins are gone.
## The library itself is always loaded, for the content (CrateContent, Decision 153)
## (NativeLibrary loads it). BREACH_NATIVE_THREADS=1 lets the Rust core part bodies on its
## thread pool (applied in a fixed order all the same). Both engines give bit-identical
## outcomes (tools/formation_digest.gd).

const GDSCRIPT := "gdscript"
const RUST := "rust"
const FIELD_CLASS := "BodyField"
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


## A new field for a battle's bodies (BodyField), whichever engine is chosen; null only
## where the Rust core isn't built, which no battle runs without (Decision 151).
static func body_field() -> Object:
	if not available(RUST):
		push_error("NativeKernels: the Rust core isn't built (native/build.sh)")
		return null
	return ClassDB.instantiate(FIELD_CLASS)


## True if body parting runs in Rust (UnitBodies): the engine chosen is "rust".
static func parts_natively() -> bool:
	return engine() == RUST


## The battle's unit store (spec 35), on its native `field`, or on a new one if none is given.
static func unit_store(field: Object = null) -> UnitStore:
	return UnitStoreNative.new(field if field != null else body_field())
