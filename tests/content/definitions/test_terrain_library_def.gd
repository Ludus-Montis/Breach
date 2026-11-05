extends GdUnitTestSuite
## TerrainLibraryDef / TerrainDef / TerrainFeatureDef, per
## specs/19-map-layout-and-objectives.md: the one shared terrain library every map uses.

const TerrainDef = preload("res://content/definitions/terrain_def.gd")
const TerrainFeatureDef = preload("res://content/definitions/terrain_feature_def.gd")
const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")


func _terrain(id: String, base: bool = true, move_cost: float = 1.0) -> TerrainDef:
	var terrain := TerrainDef.new()
	terrain.id = id
	terrain.display_name = id.capitalize()
	terrain.can_be_base = base
	terrain.move_cost = move_cost
	return terrain


func _feature(id: String) -> TerrainFeatureDef:
	var feature := TerrainFeatureDef.new()
	feature.id = id
	return feature


func _library(
	terrains: Array[TerrainDef], features: Array[TerrainFeatureDef] = []
) -> TerrainLibraryDef:
	var library := TerrainLibraryDef.new()
	library.terrains = terrains
	library.features = features
	library.road_move_multiplier = 0.5
	return library


func test_lookups_find_entries_by_id() -> void:
	var library := _library([_terrain("FIELDS"), _terrain("WATER", false)], [_feature("ORE")])

	assert_str(library.terrain("WATER").id).is_equal("WATER")
	assert_str(library.feature("ORE").id).is_equal("ORE")
	assert_object(library.terrain("LAVA")).is_null()
	assert_object(library.feature("")).is_null()
