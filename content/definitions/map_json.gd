class_name MapJson
extends RefCounted
## A MapDef as a map file in the Rust crate's schema (content/maps/<name>.json, spec 39 step
## 2c): MapFill's inverse. The map scene's export (MapSceneRoot) writes through it. Every node
## is listed once, those on lanes first, and a lane names its nodes by id; an enum is its
## name, a vector its list of numbers and a dictionary keyed by cells its [cell, value] pairs.

const STORE_CLASS := "ContentStore"


## Writes `map` to `path` (an OS path) through the crate, which checks it against the schema;
## "" when written, else why not.
static func write(map: MapDef, path: String) -> String:
	if not NativeLibrary.load_for(STORE_CLASS):
		return "the native core isn't built: run bash native/build.sh"
	return ClassDB.instantiate(STORE_CLASS).write_map(fields(map), path)


## The fields of the map file at `path` (an OS path), as the crate read it; {"error": why} if
## it didn't.
static func read(path: String) -> Dictionary:
	if not NativeLibrary.load_for(STORE_CLASS):
		return {"error": "the native core isn't built: run bash native/build.sh"}
	return ClassDB.instantiate(STORE_CLASS).read_map(path)


## `map`'s fields in the schema.
static func fields(map: MapDef) -> Dictionary:
	var nodes: Array[NodeDef] = []
	for lane in map.lanes:
		for node in lane.nodes:
			if not nodes.has(node):
				nodes.append(node)
	for node in map.off_lane_nodes:
		if not nodes.has(node):
			nodes.append(node)
	var out := _of(map)
	out.erase("off_lane_nodes")
	out["nodes"] = nodes.map(_of)
	out["lanes"] = map.lanes.map(
		func(lane: LaneDef) -> Dictionary:
			return {
				"id": lane.id,
				"nodes": lane.nodes.map(func(node: NodeDef) -> String: return node.id),
				"player_home_index": lane.player_home_index,
			}
	)
	return out


## A definition's saved fields by name.
static func _of(made: Object) -> Dictionary:
	var out := {}
	for property in made.get_property_list():
		var usage: int = property["usage"]
		if usage & PROPERTY_USAGE_SCRIPT_VARIABLE and usage & PROPERTY_USAGE_STORAGE:
			out[property["name"]] = _value(made.get(property["name"]), property)
	return out


static func _value(value: Variant, property: Dictionary) -> Variant:
	if value is Object:
		return _of(value)
	if value is Array:
		return value.map(func(item: Variant) -> Variant: return _value(item, {}))
	if value is Dictionary:
		return value.keys().map(func(key: Variant) -> Array: return [_value(key, {}), value[key]])
	if value is Vector2:
		return [value.x, value.y]
	if value is Vector2i:
		return [value.x, value.y]
	if value is Vector3i:
		return [value.x, value.y, value.z]
	if value is int and property.get("hint") == PROPERTY_HINT_ENUM:
		return _enum_name(value, property["hint_string"])
	return value


## The name of `value` in an enum's hint: a named enum's ("Origin:0,Resource:1", as Godot
## writes ORIGIN and RESOURCE) in capitals, an export_enum's ("north,west,floor") as written.
static func _enum_name(value: int, hint: String) -> String:
	var names := hint.split(",")
	for index in names.size():
		var entry := names[index]
		if ":" not in entry and index == value:
			return entry
		if ":" in entry and int(entry.get_slice(":", 1)) == value:
			return entry.get_slice(":", 0).to_upper().replace(" ", "_")
	return str(value)
