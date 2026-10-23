@tool
class_name DesignerMapImport
extends Resource
## One designer map's import settings, per specs/16-designer-map-import.md: create a
## DesignerMapImport resource (e.g. content/maps_src/<name>.import.tres), set the two
## paths, and press "Import" in the Inspector. tools/import_designer_map.gd runs the same
## path headlessly. @tool because the button handler must run in the editor, same as
## MapSceneRoot's export button.
##
## run() is the single import-validate-save path: DesignerMapImporter, then
## MapDef.validate(), and ResourceSaver.save only when there are no errors. Before
## saving it gives every sub-resource a stable id derived from its own id, so
## re-importing the same JSON produces a byte-identical .tres (a clean git diff).

const DesignerMapImporter = preload("res://content/import/designer_map_importer.gd")
const MapDef = preload("res://content/definitions/map_def.gd")

## The designer's Export JSON, saved into the repo (e.g. content/maps_src/<name>.designer.json).
@export_file("*.json") var json_path: String = ""
## Where the MapDef is written (e.g. content/maps/<name>.tres).
@export_file("*.tres") var output_tres_path: String = ""

@export_tool_button("Import") var import_now: Callable = _on_import_pressed


static func run(
	source_json_path: String, target_tres_path: String
) -> DesignerMapImporter.DesignerMapImportResult:
	var result := DesignerMapImporter.import_file(source_json_path)
	if result.errors.is_empty():
		result.errors.append_array(result.map_def.validate())
	if target_tres_path.is_empty():
		result.errors.append("output .tres path is empty")
	if not result.errors.is_empty():
		return result
	_assign_stable_ids(result.map_def)
	var save_error := ResourceSaver.save(result.map_def, target_tres_path)
	if save_error != OK:
		result.errors.append("ResourceSaver.save returned error %d" % save_error)
	return result


static func _assign_stable_ids(map_def: MapDef) -> void:
	for lane in map_def.lanes:
		lane.resource_scene_unique_id = _safe_id("Lane", lane.id)
	var seen := {}
	var all_nodes: Array = map_def.off_lane_nodes.duplicate()
	for lane in map_def.lanes:
		all_nodes.append_array(lane.nodes)
	for node in all_nodes:
		if seen.has(node):
			continue
		seen[node] = true
		node.resource_scene_unique_id = _safe_id("Node", node.id)
		for i in range(node.garrison_units.size()):
			node.garrison_units[i].resource_scene_unique_id = _safe_id(
				"Unit", "%s_%d" % [node.id, i]
			)
	for i in range(map_def.edges.size()):
		map_def.edges[i].resource_scene_unique_id = _safe_id("Edge", str(i))
	for faction in map_def.factions:
		faction.resource_scene_unique_id = _safe_id("Faction", faction.id)
	for i in range(map_def.faction_relations.size()):
		map_def.faction_relations[i].resource_scene_unique_id = _safe_id("Relation", str(i))
	for i in range(map_def.loss_groups.size()):
		map_def.loss_groups[i].resource_scene_unique_id = _safe_id("Loss", str(i))
	if map_def.layout != null:
		_assign_layout_ids(map_def.layout)


## The terrain library isn't saved with the map, so only the layout's own parts get ids.
static func _assign_layout_ids(layout) -> void:
	layout.resource_scene_unique_id = "Layout"
	for tile in layout.tiles:
		var cell_id := "%d_%d" % [tile.cell.x, tile.cell.y]
		tile.resource_scene_unique_id = _safe_id("Tile", cell_id)
		if tile.bridge != null:
			tile.bridge.resource_scene_unique_id = _safe_id("Bridge", cell_id)
	for i in range(layout.roads.size()):
		layout.roads[i].resource_scene_unique_id = _safe_id("Road", str(i))
	for i in range(layout.routes.size()):
		layout.routes[i].resource_scene_unique_id = _safe_id("Route", str(i))


## Scene-unique ids allow letters, digits and underscores only; ids like "F" and "f"
## must stay distinct, so case is kept and anything else becomes "_".
static func _safe_id(prefix: String, raw: String) -> String:
	var out := prefix + "_"
	for character in raw:
		out += (
			character if character.is_valid_ascii_identifier() or character.is_valid_int() else "_"
		)
	return out


func _on_import_pressed() -> void:
	var result := run(json_path, output_tres_path)
	for warning in result.warnings:
		push_warning("DesignerMapImport: %s" % warning)
	for error in result.errors:
		push_error("DesignerMapImport failed: %s" % error)
	if result.errors.is_empty():
		print("DesignerMapImport: wrote %s" % output_tres_path)
