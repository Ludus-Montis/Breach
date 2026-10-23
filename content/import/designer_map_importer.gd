class_name DesignerMapImporter
## Imports a Lane Tile Designer export (the designer's Export JSON) into a MapDef, per
## specs/16-designer-map-import.md. Pure and @tool-free, like MapSceneConverter:
## operates on a parsed Dictionary, so it's testable headlessly with no SceneTree.
##
## Covers everything in the export that has real schema: nodes (incl. garrison units,
## hidden_from_faction_ids, critical assets) via DesignerNodeBuilder, factions,
## relations, links (lanes + edges + off-lane nodes) via DesignerLaneDeriver, and - per
## specs/19 - the layout (grid, tiles, roads, routes; DesignerLayoutBuilder, against the
## shared terrain library) and loss groups (DesignerObjectivesBuilder). Structures are
## reported as a warning until spec 20 gives them schema.

const MapDef = preload("res://content/definitions/map_def.gd")
const FactionDef = preload("res://content/definitions/faction_def.gd")
const FactionRelationDef = preload("res://content/definitions/faction_relation_def.gd")
const DesignerNodeBuilder = preload("res://content/import/designer_node_builder.gd")
const DesignerLaneDeriver = preload("res://content/import/designer_lane_deriver.gd")
const DesignerLayoutBuilder = preload("res://content/import/designer_layout_builder.gd")
const DesignerObjectivesBuilder = preload("res://content/import/designer_objectives_builder.gd")
const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")

const FORMAT := "breach-designer-map"
const FORMAT_VERSION := 1
## The designer has no sim tuning; p_f_F_c's values are the defaults unless the JSON
## carries a "sim" object.
const DEFAULT_SIM := {
	"tick_duration_seconds": 1.5,
	"suspicion_tier_thresholds": [20, 45, 70, 90],
	"suspicion_decay_per_tick": 2,
}
const NOT_YET_IMPORTED := ["static_defense_library", "structure_feature_library"]


class DesignerMapImportResult:
	extends RefCounted
	var map_def: MapDef = MapDef.new()
	var errors: PackedStringArray = PackedStringArray()
	var warnings: PackedStringArray = PackedStringArray()


static func import_file(json_path: String) -> DesignerMapImportResult:
	if not FileAccess.file_exists(json_path):
		var missing := DesignerMapImportResult.new()
		missing.errors.append("designer export not found: %s" % json_path)
		return missing
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(json_path))
	if typeof(parsed) != TYPE_DICTIONARY:
		var bad := DesignerMapImportResult.new()
		bad.errors.append("designer export is not a JSON object: %s" % json_path)
		return bad
	return import_map(parsed)


## library defaults to the shared one the crate loaded (CrateContent.terrain_library()).
static func import_map(
	export_data: Dictionary, library: TerrainLibraryDef = null
) -> DesignerMapImportResult:
	var result := DesignerMapImportResult.new()
	if (
		export_data.get("format") != FORMAT
		or int(export_data.get("format_version", -1)) != FORMAT_VERSION
	):
		result.errors.append(
			(
				"unsupported export format: expected '%s' version %d, got '%s' version %s"
				% [
					FORMAT,
					FORMAT_VERSION,
					export_data.get("format"),
					export_data.get("format_version")
				]
			)
		)
		return result
	_apply_sim_scalars(export_data.get("sim", {}), result.map_def)
	result.map_def.factions = _build_factions(export_data.get("factions", []))
	result.map_def.faction_relations = _build_relations(
		export_data.get("faction_relations", []), result.errors
	)
	var nodes := DesignerNodeBuilder.build_nodes(export_data, result.errors, result.warnings)
	DesignerLaneDeriver.derive(export_data, nodes, result.map_def, result.errors)
	if not export_data.get("grid", {}).is_empty():
		if library == null:
			library = CrateContent.terrain_library()
		result.map_def.layout = DesignerLayoutBuilder.build(export_data, library, result.errors)
	result.map_def.loss_groups = DesignerObjectivesBuilder.build_loss_groups(
		export_data, result.errors
	)
	_warn_not_imported(export_data, result.warnings)
	return result


static func _apply_sim_scalars(sim: Dictionary, map_def: MapDef) -> void:
	map_def.tick_duration_seconds = float(
		sim.get("tick_duration_seconds", DEFAULT_SIM["tick_duration_seconds"])
	)
	var thresholds: Array[int] = []
	for value in sim.get("suspicion_tier_thresholds", DEFAULT_SIM["suspicion_tier_thresholds"]):
		thresholds.append(int(value))
	map_def.suspicion_tier_thresholds = thresholds
	map_def.suspicion_decay_per_tick = int(
		sim.get("suspicion_decay_per_tick", DEFAULT_SIM["suspicion_decay_per_tick"])
	)


static func _build_factions(factions_data: Array) -> Array[FactionDef]:
	var factions: Array[FactionDef] = []
	for entry in factions_data:
		var faction := FactionDef.new()
		faction.id = DesignerNodeBuilder.text(entry.get("id"))
		faction.display_name = DesignerNodeBuilder.text(entry.get("display_name"))
		factions.append(faction)
	return factions


static func _build_relations(
	relations_data: Array, errors: PackedStringArray
) -> Array[FactionRelationDef]:
	var relations: Array[FactionRelationDef] = []
	for entry in relations_data:
		var relation := FactionRelationDef.new()
		relation.faction_a_id = DesignerNodeBuilder.text(entry.get("a"))
		relation.faction_b_id = DesignerNodeBuilder.text(entry.get("b"))
		var stance := DesignerNodeBuilder.text(entry.get("stance"))
		if FactionRelationDef.Stance.has(stance):
			relation.stance = FactionRelationDef.Stance[stance]
		else:
			errors.append(
				(
					"relation %s-%s: unknown stance '%s'"
					% [relation.faction_a_id, relation.faction_b_id, stance]
				)
			)
		relations.append(relation)
	return relations


static func _warn_not_imported(export_data: Dictionary, warnings: PackedStringArray) -> void:
	var skipped: Array[String] = []
	for section in NOT_YET_IMPORTED:
		if not export_data.get(section, []).is_empty():
			skipped.append(section)
	if export_data.get("nodes", []).any(func(n): return n.has("structure")):
		skipped.append("structures")
	if not skipped.is_empty():
		warnings.append("not imported yet (spec 20): " + ", ".join(skipped))
