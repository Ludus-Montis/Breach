class_name NativeKernels
extends RefCounted
## The battle's native state (Decision 129, native/README.md): every battle has its bodies
## on a native BodyField and its unit store there, and the passes run on it. The last
## GDScript twin, body parting's, retired in spec 39 step 6a, and the engine switch
## (BREACH_NATIVE) with it. The library is always loaded, for the content too
## (CrateContent, Decision 153; NativeLibrary loads it). BREACH_NATIVE_THREADS=1 lets the
## Rust core part bodies on its thread pool (applied in a fixed order all the same).

const FIELD_CLASS := "BodyField"
const UnitStoreNative = preload("res://sim/skirmish/formation/unit_store_native.gd")

## Whether the native core may use threads (Rayon's pool): BREACH_NATIVE_THREADS's, unless
## a bench sets it.
static var threaded := OS.get_environment("BREACH_NATIVE_THREADS") == "1"


## True once the Rust core's library is built and loaded.
static func available() -> bool:
	return NativeLibrary.load_for(FIELD_CLASS)


## A new field for a battle's bodies (BodyField); null only where the Rust core isn't
## built, which no battle runs without (Decision 151).
static func body_field() -> Object:
	if not available():
		push_error("NativeKernels: the Rust core isn't built (native/build.sh)")
		return null
	return ClassDB.instantiate(FIELD_CLASS)


## The battle's unit store (spec 35), on its native `field`, or on a new one if none is given.
static func unit_store(field: Object = null) -> UnitStore:
	return UnitStoreNative.new(field if field != null else body_field())
