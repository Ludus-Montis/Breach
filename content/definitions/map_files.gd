class_name MapFiles
extends RefCounted
## A map file (content/maps/<name>.json, spec 39 step 2c) read fresh or written, through the
## Rust crate, which checks it against the schema and the map's rules
## (native/rust/src/content/map_check.rs). The map viewer reads with it, so Reload shows what
## the designer last saved; the map scene's export writes with it.


## The MapDef in the file at `path` (an OS path), or {"error": why it didn't load}.
static func read(path: String) -> Dictionary:
	var store := CrateContent.loaded_store()
	if store == null:
		return {"error": "the native core isn't built: run bash native/build.sh"}
	CrateContent.terrain_library()
	return store.read_map(path)


## Writes `map` to `path` (an OS path); "" when written, else why not.
static func write(map: MapDef, path: String) -> String:
	var store := CrateContent.loaded_store()
	if store == null:
		return "the native core isn't built: run bash native/build.sh"
	return store.write_map(map, path)
