extends GdUnitTestSuite
## Elevation in the map data, per Decision 56: a tile's elevation (cells) overrides its
## terrain's default, and the map has a ceiling.

const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const TileDef = preload("res://content/definitions/tile_def.gd")
const TerrainDef = preload("res://content/definitions/terrain_def.gd")
const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")


func _layout() -> MapLayoutDef:
	var hills := TerrainDef.new()
	hills.id = "HILLY"
	hills.default_elevation = 8
	var library := TerrainLibraryDef.new()
	library.terrains = [hills]
	var layout := MapLayoutDef.new()
	layout.cols = 3
	layout.rows = 1
	layout.default_terrain_id = "HILLY"
	layout.terrain_library = library
	var tile := TileDef.new()
	tile.cell = Vector2i(1, 0)
	tile.elevation = 20
	layout.tiles = [tile]
	return layout


func test_a_tile_without_an_elevation_takes_its_terrains_default() -> void:
	assert_int(_layout().elevation_at(Vector2i(0, 0))).is_equal(8)


func test_a_tiles_elevation_overrides_its_terrains() -> void:
	assert_int(_layout().elevation_at(Vector2i(1, 0))).is_equal(20)


func test_the_ceiling_defaults_to_four_tiles() -> void:
	assert_int(MapLayoutDef.new().ceiling).is_equal(64)
