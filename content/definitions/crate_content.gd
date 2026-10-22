class_name CrateContent
extends RefCounted
## The simulation's content as the Rust crate loaded it (Decision 153, spec 39 step 2): units,
## weapons, armour and the battle's tuning, JSON under content/ in the crate's schema
## (content/schema/). GDScript's definitions (UnitDef, WeaponDef, ArmourDef, BattleTuning)
## are filled from it, field for field, until their last reader ports to Rust; nothing reads
## them from .tres. Each is made once and shared, as load() shared a .tres: duplicate() one
## before changing it. The core must be built (bash native/build.sh).

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
		fill(made, fields)
		made.items.assign(items.map(item))
		_made[key] = made
	return _made[key]


## The item at `path`, under content/ without .json: weapons/<id> or armour/<id>.
static func item(path: String) -> ItemDef:
	if not _made.has(path):
		var kind := path.get_slice("/", 0)
		var made: ItemDef = WeaponDef.new() if kind == "weapons" else ArmourDef.new()
		_made[path] = fill(made, _fields(kind, path.get_slice("/", 1)))
	return _made[path]


## The weapon `id` (content/weapons/<id>.json).
static func weapon(id: String) -> WeaponDef:
	return item("weapons/" + id)


## The armour `id` (content/armour/<id>.json).
static func armour(id: String) -> ArmourDef:
	return item("armour/" + id)


## The battle's tuning (content/tuning/battle_tuning.json).
static func tuning() -> BattleTuning:
	if not _made.has("tuning"):
		_made["tuning"] = fill(BattleTuning.new(), _content().get("tuning", {}))
	return _made["tuning"]


## The crate's fields for each kind when a file names none ({"unit", "weapon", "armour"}).
static func defaults() -> Dictionary:
	if not NativeLibrary.load_for(STORE_CLASS):
		return {}
	return ClassDB.instantiate(STORE_CLASS).defaults()


## Sets each of `fields` on `made` by name and returns it. An item's shared fields ("item")
## are its own; a typed list keeps its type ("inf" in a list of reals is an infinity); a
## name where an enum is kept is the enum's value.
static func fill(made: Object, fields: Dictionary) -> Object:
	for key: String in fields:
		var value: Variant = fields[key]
		var current: Variant = made.get(key)
		if key == "item" and value is Dictionary:
			fill(made, value)
		elif current is Array:
			var reals: bool = (current as Array).get_typed_builtin() == TYPE_FLOAT
			(current as Array).assign((value as Array).map(_real) if reals else value)
		elif current is int and value is String:
			made.set(key, _enum_value(made, key, value))
		else:
			made.set(key, value)
	return made


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


static func _real(value: Variant) -> float:
	if value is String:
		return -INF if value == "-inf" else INF
	return value


## The value of `made`'s enum `key` named `named` (case aside), from its export's hint.
static func _enum_value(made: Object, key: String, named: String) -> int:
	for property in made.get_property_list():
		if property["name"] == key:
			var names: PackedStringArray = property["hint_string"].split(",")
			for index in names.size():
				var entry := names[index]
				if entry.get_slice(":", 0).to_lower() == named.to_lower():
					return int(entry.get_slice(":", 1)) if ":" in entry else index
	push_error("CrateContent: %s isn't one of %s's values" % [named, key])
	return 0
