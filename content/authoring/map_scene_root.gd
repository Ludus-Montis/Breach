@tool
class_name MapSceneRoot
extends Node2D
## Root of a hand-authored map scene, per specs/10-map-scene-authoring.md. Children
## (in order) are MapLaneRoots. The "Export to JSON" button in the Inspector converts
## this scene's topology (via MapSceneConverter) plus this node's own map-global
## scalar fields into a MapDef and writes it to output_path, a map file in the Rust crate's
## schema (MapJson, spec 39 step 2c) - the only place this tool writes to
## content/maps/*.json. Engine-glue-only, same disclosed-limitation
## convention as LaneView: a construction smoke test is the only automated coverage
## (tests/content/authoring/test_map_scene_root.gd); the export/save path itself is
## manual-playtest-verified.
##
## This is the first project-authored @tool script in this repo - @tool is required
## because export_now's handler must run in the editor, not just at game runtime.
## MapLaneRoot/MapNodeMarker deliberately stay non-@tool: they only need their @export
## fields to render in the Inspector, which any script gets for free at edit time.

const MapSceneConverter = preload("res://content/authoring/map_scene_converter.gd")
const MapDef = preload("res://content/definitions/map_def.gd")

@export var tick_duration_seconds: float = 0.0
@export var suspicion_tier_thresholds: Array[int] = []
@export var suspicion_decay_per_tick: int = 0
@export var output_path: String = ""

@export_tool_button("Export to JSON") var export_now: Callable = _on_export_pressed


func _on_export_pressed() -> void:
	var result := MapSceneConverter.build_map_def(self)
	var map_def: MapDef = result.map_def
	map_def.tick_duration_seconds = tick_duration_seconds
	map_def.suspicion_tier_thresholds = suspicion_tier_thresholds
	map_def.suspicion_decay_per_tick = suspicion_decay_per_tick

	var errors := PackedStringArray(result.errors)
	errors.append_array(map_def.validate())

	if not errors.is_empty():
		for error in errors:
			push_error("MapSceneRoot export failed: %s" % error)
		return

	if output_path.is_empty():
		push_error("MapSceneRoot export failed: output_path is empty")
		return

	var write_error := MapJson.write(map_def, ProjectSettings.globalize_path(output_path))
	if not write_error.is_empty():
		push_error("MapSceneRoot export failed: %s" % write_error)
