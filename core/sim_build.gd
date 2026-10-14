class_name SimBuild
extends RefCounted
## Which build of the simulation is running (Decision 130): a fingerprint of everything that
## decides a battle - the scripts under res://sim/ and the content (unit types, items,
## tuning) under res://content/ - so a record of play can say which build played it, and a
## replay can say when it isn't this one. Not the commit: an uncommitted edit changes
## outcomes, a docs-only commit doesn't. Line endings are ignored, so a checkout with CRLF
## (git on Windows) has the same build as one with LF. Twelve hex digits; "" where the
## sources can't be read (an exported game ships compiled scripts).

const ROOTS := ["res://sim", "res://content"]
const KINDS := ["gd", "tres", "tscn", "json", "cfg"]

static var _id := ""
static var _known := false


## This build's fingerprint, worked out once.
static func id() -> String:
	if not _known:
		_id = _fingerprint()
		_known = true
	return _id


## "" if a record stamped `recorded` was played on this build; otherwise a warning that its
## replay may play out differently.
static func compare(recorded: String) -> String:
	var here := id()
	if recorded == "":
		return "This record has no build stamp: it may play out differently on build %s." % here
	if recorded == here:
		return ""
	return "Recorded on build %s; this is build %s: it may play out differently." % [recorded, here]


static func _fingerprint() -> String:
	var files := []
	for root in ROOTS:
		_collect(root, files)
	if files.is_empty():
		return ""
	files.sort()
	var digest := HashingContext.new()
	digest.start(HashingContext.HASH_SHA256)
	for path in files:
		digest.update(path.to_utf8_buffer())
		digest.update(FileAccess.get_file_as_string(path).replace("\r", "").to_utf8_buffer())
	return digest.finish().hex_encode().substr(0, 12)


static func _collect(folder: String, out: Array) -> void:
	var dir := DirAccess.open(folder)
	if dir == null:
		return
	for file in dir.get_files():
		if file.get_extension() in KINDS:
			out.append(folder.path_join(file))
	for child in dir.get_directories():
		_collect(folder.path_join(child), out)
