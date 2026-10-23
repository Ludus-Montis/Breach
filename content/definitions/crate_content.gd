class_name CrateContent
extends RefCounted
## The simulation's content as the Rust crate loaded it (Decision 153, spec 39 step 2): units,
## weapons, armour, the battle's tuning, the terrain library and the maps, JSON under content/
## in the crate's schema (content/schema/). GDScript's definitions (UnitDef, WeaponDef,
## ArmourDef, BattleTuning, TerrainLibraryDef, MapDef and what they hold) are filled from it,
## field for field, until their last reader ports to Rust; nothing reads them from .tres. Each
## is made once and shared, as load() shared a .tres: duplicate() one before changing it. The
## core must be built (bash native/build.sh).

const STORE_CLASS := "ContentStore"
const ROOT := "res://content"

static var _loaded: Dictionary = {}
static var _made: Dictionary = {}


## The unit `id` (content/units/<id>.json).
static func unit(id: String) -> UnitDef:
	var key := "units/" + id
	if not _made.has(key):
		var fields: Dictionary = _fields("units", id).duplicate()
		var made := UnitDef.new()
		var items: Array = fields.get("items", [])
		fields.erase("items")
		ContentFill.fill(made, fields)
		made.items.assign(items.map(_item))
		_made[key] = made
	return _made[key]


## The item at `path`, under content/ without .json: weapons/<id> or armour/<id>.
static func _item(path: String) -> ItemDef:
	if not _made.has(path):
		var kind := path.get_slice("/", 0)
		var made: ItemDef = WeaponDef.new() if kind == "weapons" else ArmourDef.new()
		_made[path] = ContentFill.fill(made, _fields(kind, path.get_slice("/", 1)))
	return _made[path]


## The weapon `id` (content/weapons/<id>.json).
static func weapon(id: String) -> WeaponDef:
	return _item("weapons/" + id)


## The armour `id` (content/armour/<id>.json).
static func armour(id: String) -> ArmourDef:
	return _item("armour/" + id)


## The battle's tuning (content/tuning/battle_tuning.json).
static func tuning() -> BattleTuning:
	if not _made.has("tuning"):
		_made["tuning"] = ContentFill.fill(BattleTuning.new(), _content().get("tuning", {}))
	return _made["tuning"]


## The one shared terrain library (content/terrain/terrain_library.json, Decision 32), which
## every map's layout references.
static func terrain_library() -> TerrainLibraryDef:
	if not _made.has("terrain"):
		_made["terrain"] = ContentFill.fill(TerrainLibraryDef.new(), _content().get("terrain", {}))
	return _made["terrain"]


## The map `name` (content/maps/<name>.json).
static func map(name: String) -> MapDef:
	var key := "maps/" + name
	if not _made.has(key):
		_made[key] = MapFill.build(_fields("maps", name))
	return _made[key]


## The crate's fields for each kind when a file names none ({"unit", "weapon", "armour"}).
static func defaults() -> Dictionary:
	if not NativeLibrary.load_for(STORE_CLASS):
		return {}
	return ClassDB.instantiate(STORE_CLASS).defaults()


## One kind's fields for `id`, or none (with an error) if it isn't there.
static func _fields(kind: String, id: String) -> Dictionary:
	var of_kind: Dictionary = _content().get(kind, {})
	if not of_kind.has(id):
		push_error("CrateContent: no %s/%s.json under %s" % [kind, id, ROOT])
		return {}
	return of_kind[id]


## Everything the crate loaded from content/, loaded once.
static func _content() -> Dictionary:
	if _loaded.is_empty():
		if not NativeLibrary.load_for(STORE_CLASS):
			push_error("CrateContent: the native core isn't built: run bash native/build.sh")
			return {}
		var root := ProjectSettings.globalize_path(ROOT)
		_loaded = ClassDB.instantiate(STORE_CLASS).load(root)
		if _loaded.has("error"):
			push_error("CrateContent: %s" % _loaded["error"])
	return _loaded
