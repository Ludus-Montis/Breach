class_name CrateContent
extends RefCounted
## The simulation's content as the Rust crate loaded it (Decision 153, spec 39 step 2): units,
## weapons, armour, the battle's tuning, the terrain library and the maps, JSON under content/
## in the crate's schema (content/schema/). The crate hands GDScript its definitions (UnitDef,
## WeaponDef, ArmourDef, BattleTuning, TerrainLibraryDef, MapDef and what they hold), filled
## field for field (ContentStore, native/rust/src/lib.rs), until their last reader ports to
## Rust; nothing reads them from .tres. Each is made once and shared, as load() shared a
## .tres: duplicate() one before changing it. The core must be built (bash native/build.sh).

const STORE_CLASS := "ContentStore"
const ROOT := "res://content"

static var _store_made: Object = null
static var _library: TerrainLibraryDef = null


## The unit `id` (content/units/<id>.json).
static func unit(id: String) -> UnitDef:
	return _store().unit(id)


## The weapon `id` (content/weapons/<id>.json).
static func weapon(id: String) -> WeaponDef:
	return _store().weapon(id)


## The battle's tuning (content/tuning/battle_tuning.json).
static func tuning() -> BattleTuning:
	return _store().tuning()


## The one shared terrain library (content/terrain/terrain_library.json, Decision 32), which
## every map's layout references. Kept here, so a layout made while the crate makes a map
## doesn't call back into it.
static func terrain_library() -> TerrainLibraryDef:
	if _library == null:
		_library = _store().terrain_library()
	return _library


## The map `name` (content/maps/<name>.json).
static func map(name: String) -> MapDef:
	terrain_library()
	return _store().map(name)


## A fresh definition of each kind ({"UnitDef": ..., "MapDef": ...}) filled with the crate's
## defaults, for the test that holds them equal to GDScript's.
static func defaults() -> Dictionary:
	return _store().defaults() if _store() else {}


## The crate's store, with the content under content/ loaded once.
static func _store() -> Object:
	if _store_made == null:
		_store_made = loaded_store()
	return _store_made


## A store of the crate's with the content loaded; null, with an error, if the core isn't
## built or the content didn't load.
static func loaded_store() -> Object:
	if not NativeLibrary.load_for(STORE_CLASS):
		push_error("CrateContent: the native core isn't built: run bash native/build.sh")
		return null
	var store: Object = ClassDB.instantiate(STORE_CLASS)
	var failed: String = store.load(ProjectSettings.globalize_path(ROOT))
	if not failed.is_empty():
		push_error("CrateContent: %s" % failed)
	return store
