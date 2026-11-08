class_name FieldCapture
extends RefCounted
## The 2D field's capture (spec 44 step 2c, Decision 169). Started with `-- --capture=<name>`,
## FormationFieldScene plays that scenario from content/scenarios/ hands-free. Each frame plays
## every tick its clock owes and waits for them, so a movie's frames are the same on every run.
## When the scenario ends (FieldState.ended: a side broken and its rout played out, or 90 s of
## play) it writes capture.json (`scenarios::capture`: the moments, the behaviours seen) and
## quits. capture.json goes beside the movie (`--write-movie <path>`), or where
## `--capture-out=<path>` says, else to user://. The commit is BREACH_COMMIT's, else
## GITHUB_SHA's. The window is SIZE. A name not on the list prints an error naming it and
## quits with 2. Step 3's workflow runs it:
##
##   godot --path . --resolution 1280x720 --fixed-fps 30 --write-movie /tmp/c/frame.png \
##     res://presentation/skirmish/formation_2d/formation_field.tscn -- --capture=field_break_off

const ASK := "--capture="
const PLACE := "--capture-out="
const MOVIE := "--write-movie"
const FILE := "capture.json"
const SIZE := Vector2i(1280, 720)

var name := ""  # the scenario's
var out := ""  # where capture.json is written
var quits := true  # false in tests: the tree stays up when the capture ends
var written := false


func _init(scenario: String = "", path: String = "") -> void:
	name = scenario
	out = path


## The content folder the scenario list is under, as a path the crate reads.
static func root() -> String:
	return ProjectSettings.globalize_path("res://content")


## The capture this run's command line asks for, else null.
static func asked() -> FieldCapture:
	return requested(OS.get_cmdline_args() + OS.get_cmdline_user_args())


## The capture `args` ask for (`--capture=<name>`), else null.
static func requested(args: PackedStringArray) -> FieldCapture:
	var scenario := ""
	var placed := ""
	var beside := "user://" + FILE
	for i in args.size():
		if args[i].begins_with(ASK):
			scenario = args[i].trim_prefix(ASK)
		elif args[i].begins_with(PLACE):
			placed = args[i].trim_prefix(PLACE)
		elif args[i] == MOVIE and i + 1 < args.size():
			beside = args[i + 1].get_base_dir().path_join(FILE)
	if scenario == "":
		return null
	return FieldCapture.new(scenario, placed if placed != "" else beside)


## The scenario opened on `scene` (FormationFieldScene.restart) and the window sized;
## false if it can't be, the error printed and, unless in a test, the tree quit with 2.
func start(scene: Node) -> bool:
	var wrong: String = scene.restart(false, 0, [], name)
	if wrong != "":
		printerr(wrong)
		if quits:
			scene.get_tree().quit(2)
		return false
	if quits and DisplayServer.get_name() != "headless":
		scene.get_window().size = SIZE
	return true


## A frame owing `due` ticks: each played on `scene`'s field and waited for, until the scenario
## ends; their events. Once it has ended, capture.json is written and the tree quits.
func frame(scene: Node, due: int) -> Array:
	var field: Object = scene.field()
	var events: Array = []
	for i in due:
		if field.ended():
			break
		events.append_array(field.run_ticks(1))
	if field.ended() and not written:
		_write(field.capture(_commit()))
		written = true
		if quits:
			scene.get_tree().quit()
	return events


func _write(text: String) -> void:
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var file := FileAccess.open(out, FileAccess.WRITE)
	file.store_string(text + "\n")


static func _commit() -> String:
	var commit := OS.get_environment("BREACH_COMMIT")
	return commit if commit != "" else OS.get_environment("GITHUB_SHA")
