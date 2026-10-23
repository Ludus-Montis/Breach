extends GdUnitTestSuite
## Layout and objectives import, per specs/19-map-layout-and-objectives.md, against a
## real designer export (tests/fixtures/designer/layout_map.designer.json) and the
## committed shared terrain library.

const DesignerMapImporter = preload("res://content/import/designer_map_importer.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const LossGroupDef = preload("res://content/definitions/loss_group_def.gd")

const LAYOUT_PATH := "res://tests/fixtures/designer/layout_map.designer.json"


func _export() -> Dictionary:
	return JSON.parse_string(FileAccess.get_file_as_string(LAYOUT_PATH))


func _import(export_data: Dictionary = _export()):
	return DesignerMapImporter.import_map(export_data)


func _node(map_def, id: String):
	for lane in map_def.lanes:
		for node in lane.nodes:
			if node.id == id:
				return node
	for node in map_def.off_lane_nodes:
		if node.id == id:
			return node
	return null


func _any(messages: PackedStringArray, needle: String) -> bool:
	return Array(messages).any(func(m): return m.contains(needle))


func test_the_layout_carries_the_grid_and_references_the_shared_library() -> void:
	var result = _import()

	assert_array(result.errors).is_empty()
	var layout: MapLayoutDef = result.map_def.layout
	assert_int(layout.cols).is_equal(12)
	assert_int(layout.rows).is_equal(5)
	assert_int(layout.cell_size).is_equal(64)
	assert_str(layout.default_terrain_id).is_equal("FIELDS")
	assert_object(layout.terrain_library).is_same(CrateContent.terrain_library())


func test_tiles_import_terrain_feature_bridge_and_upgrades() -> void:
	var layout: MapLayoutDef = _import().map_def.layout

	assert_int(layout.tiles.size()).is_equal(6)
	assert_str(layout.tile_at(Vector2i(2, 3)).terrain_id).is_equal("FOREST")
	assert_str(layout.tile_at(Vector2i(9, 4)).feature_id).is_equal("ORE_VEIN")
	var farm := layout.tile_at(Vector2i(4, 2))
	assert_str(farm.terrain_id).is_equal("")  # the designer's "base terrain" flag
	assert_int(farm.upgrade_slots).is_equal(1)
	assert_array(farm.upgrade_ids).is_equal(["GUARD_BARRACKS"])
	assert_int(farm.stability).is_equal(-1)
	var bridge := layout.tile_at(Vector2i(7, 3)).bridge
	assert_object(bridge).is_not_null()
	assert_int(bridge.hp).is_equal(40)


func test_roads_and_routes_import() -> void:
	var layout: MapLayoutDef = _import().map_def.layout

	assert_int(layout.roads.size()).is_equal(7)
	assert_object(layout.roads[0].a).is_equal(Vector2i(1, 2))
	assert_int(layout.routes.size()).is_equal(6)
	var first = layout.routes[0]
	assert_str(first.label()).is_equal("p-f")
	assert_object(first.cells[0]).is_equal(Vector2i(1, 2))
	assert_object(first.cells[-1]).is_equal(Vector2i(4, 2))
	assert_float(first.cost).is_equal(1.5)


func test_critical_assets_and_loss_groups_import() -> void:
	var map_def = _import().map_def

	assert_bool(_node(map_def, "p").is_critical_asset).is_true()
	assert_bool(_node(map_def, "c").is_critical_asset).is_true()
	assert_bool(_node(map_def, "f").is_critical_asset).is_false()
	assert_int(map_def.loss_groups.size()).is_equal(2)
	var home: LossGroupDef = map_def.loss_groups[0]
	assert_str(home.faction_id).is_equal("player")
	assert_str(home.display_name).is_equal("Home")
	assert_int(home.rule).is_equal(LossGroupDef.Rule.ANY)
	assert_array(home.node_ids).is_equal(["p"])


func test_the_imported_map_validates() -> void:
	var result = _import()

	assert_array(result.errors).is_empty()
	assert_array(result.map_def.validate()).is_empty()


func test_only_structure_sections_are_still_not_imported() -> void:
	var warnings: PackedStringArray = _import().warnings

	assert_bool(_any(warnings, "structures")).is_true()
	for section in ["tiles", "roads", "terrain_library", "loss_criteria", "link routes"]:
		assert_bool(_any(warnings, section)).is_false()


func test_a_terrain_missing_from_the_shared_library_is_an_error() -> void:
	var export_data := _export()
	export_data["tiles"][0]["terrain"] = "NOT_A_TERRAIN"

	var errors: PackedStringArray = _import(export_data).errors

	(
		assert_bool(
			_any(
				errors, "tile (2, 3): terrain 'NOT_A_TERRAIN' is not in the shared terrain library"
			)
		)
		. is_true()
	)


func test_an_unknown_loss_rule_is_an_error() -> void:
	var export_data := _export()
	export_data["loss_criteria"][0]["rule"] = "SOME"

	assert_bool(_any(_import(export_data).errors, "unknown rule 'SOME'")).is_true()


func test_an_export_without_a_grid_has_no_layout() -> void:
	var export_data := _export()
	export_data.erase("grid")
	export_data.erase("tiles")
	export_data.erase("roads")

	assert_object(_import(export_data).map_def.layout).is_null()
