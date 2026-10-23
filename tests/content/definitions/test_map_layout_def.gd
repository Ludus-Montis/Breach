extends GdUnitTestSuite
## MapLayoutDef and TileDef, per
## specs/19-map-layout-and-objectives.md: the lookups presentation and the ground read. The
## layout's rules are the crate's (native/rust/src/content/map_check_tests.rs).

const TerrainDef = preload("res://content/definitions/terrain_def.gd")
const TerrainFeatureDef = preload("res://content/definitions/terrain_feature_def.gd")
const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")
const TileDef = preload("res://content/definitions/tile_def.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")


func _terrain(id: String, base: bool, needs_bridge: bool, stability: int) -> TerrainDef:
	var terrain := TerrainDef.new()
	terrain.id = id
	terrain.can_be_base = base
	terrain.needs_bridge = needs_bridge
	terrain.default_stability = stability
	terrain.default_max_width = 1
	return terrain


func _library() -> TerrainLibraryDef:
	var library := TerrainLibraryDef.new()
	var terrains: Array[TerrainDef] = [
		_terrain("FIELDS", true, false, 4),
		_terrain("FOREST", true, false, 2),
		_terrain("WATER", false, true, 0),
	]
	library.terrains = terrains
	var ore := TerrainFeatureDef.new()
	ore.id = "ORE"
	var features: Array[TerrainFeatureDef] = [ore]
	library.features = features
	return library


func _tile(col: int, row: int, terrain_id: String = "") -> TileDef:
	var tile := TileDef.new()
	tile.cell = Vector2i(col, row)
	tile.terrain_id = terrain_id
	return tile


func _layout(tiles: Array[TileDef] = []) -> MapLayoutDef:
	var layout := MapLayoutDef.new()
	layout.cols = 6
	layout.rows = 4
	layout.default_terrain_id = "FIELDS"
	layout.terrain_library = _library()
	layout.tiles = tiles
	return layout


func test_terrain_at_falls_back_to_the_default_terrain() -> void:
	var layout := _layout([_tile(1, 1, "FOREST")] as Array[TileDef])

	assert_str(layout.terrain_id_at(Vector2i(1, 1))).is_equal("FOREST")
	assert_str(layout.terrain_id_at(Vector2i(3, 3))).is_equal("FIELDS")
	assert_object(layout.tile_at(Vector2i(1, 1))).is_not_null()
	assert_object(layout.tile_at(Vector2i(3, 3))).is_null()


func test_effective_capacity_uses_overrides_and_terrain_defaults() -> void:
	var tile := _tile(1, 1, "FOREST")
	tile.max_width = 4

	var capacity := tile.effective_capacity(_library(), "FIELDS")

	assert_int(capacity["stability"]).is_equal(2)
	assert_int(capacity["max_width"]).is_equal(4)
