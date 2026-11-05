class_name ContentRegistry
extends RefCounted
## The registry of tags, traits and statuses (Decision 128), read from content/registry/*.json
## - the one place each is defined, described and tagged, which the designer's Library
## browses. A trait's tags say where it may sit (each tag's targets: unit, leader, weapon,
## armour, tool, material, structure); it may sit where all its tags agree, narrowed by
## its "only". A trait may carry a "default" level every unit has without listing it (an
## implicit trait: climber 0, swimmer 0; spec 30). The crate's loader holds the content to
## it (`native/rust/src/content/checks.rs`, spec 39 step 9e-i). Pure, cached.

const DIR := "res://content/registry/"
const KINDS := ["tags", "traits", "statuses"]

static var _cache := {}
static var _implicit := {}


## {"tags": {id: entry}, "traits": {id: entry}, "statuses": {id: entry}}.
static func current() -> Dictionary:
	if _cache.is_empty():
		for kind in KINDS:
			var read = JSON.parse_string(FileAccess.get_file_as_string(DIR + kind + ".json"))
			_cache[kind] = {}
			for entry in read[kind] if read is Dictionary else []:
				_cache[kind][entry["id"]] = entry
	return _cache


## The implicit traits, id -> the level every unit has without listing them.
static func implicit() -> Dictionary:
	if _implicit.is_empty():
		for trait_id in current()["traits"]:
			var entry: Dictionary = current()["traits"][trait_id]
			if entry.has("default"):
				_implicit[trait_id] = int(entry["default"])
	return _implicit
