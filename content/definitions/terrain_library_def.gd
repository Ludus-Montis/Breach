class_name TerrainLibraryDef
extends Resource
## The one shared terrain library (Decision 32, specs/19-map-layout-and-objectives.md):
## content/terrain/terrain_library.json in the crate's schema, which the designer edits and
## CrateContent.terrain_library() fills this from (spec 39 step 2b). Every map's MapLayoutDef
## references that one library rather than copying it, so a terrain edit reaches every map
## without re-saving them.

const TerrainDef = preload("res://content/definitions/terrain_def.gd")
const TerrainFeatureDef = preload("res://content/definitions/terrain_feature_def.gd")
const MaterialDef = preload("res://content/definitions/material_def.gd")

@export var terrains: Array[TerrainDef] = []
@export var features: Array[TerrainFeatureDef] = []
## What strata (and later walls) are made of (Decisions 54, 57).
@export var materials: Array[MaterialDef] = []
## Multiplies a terrain's move_cost on a cell with a road.
@export var road_move_multiplier: float = 0.5


func terrain(terrain_id: String) -> TerrainDef:
	for entry in terrains:
		if entry.id == terrain_id:
			return entry
	return null


func material(material_id: String) -> MaterialDef:
	for entry in materials:
		if entry.id == material_id:
			return entry
	return null


func feature(feature_id: String) -> TerrainFeatureDef:
	for entry in features:
		if entry.id == feature_id:
			return entry
	return null
