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


func validate() -> PackedStringArray:
	var errors := PackedStringArray()
	errors.append_array(_validate_ids(terrains, "terrain"))
	errors.append_array(_validate_ids(features, "feature"))
	errors.append_array(_validate_ids(materials, "material"))
	for entry in materials:
		errors.append_array(_validate_transitions(entry))
	if not terrains.any(func(t): return t.can_be_base):
		errors.append("at least one terrain must have can_be_base, so a map has a base terrain")
	for entry in terrains:
		if entry.move_cost < 0.0:
			errors.append(
				"terrain '%s': move_cost must be >= 0, got %f" % [entry.id, entry.move_cost]
			)
		errors.append_array(_validate_ground(entry))
	if road_move_multiplier < 0.0:
		errors.append("road_move_multiplier must be >= 0, got %f" % road_move_multiplier)
	return errors


func _validate_ground(entry: TerrainDef) -> PackedStringArray:
	var errors := PackedStringArray()
	var at := "terrain '%s': " % entry.id
	if entry.foundation_max < entry.bearing:
		errors.append(at + "foundation_max %d < bearing %d" % [entry.foundation_max, entry.bearing])
	for body in entry.liquids:
		var liquid := material(body.material_id)
		if liquid == null:
			errors.append(at + "liquid body of unknown material '%s'" % body.material_id)
		elif not liquid.is_liquid():
			errors.append(at + "liquid body of '%s' doesn't flow" % body.material_id)
		if body.min_cells < 0 or body.min_cells > body.max_cells:
			errors.append(at + "liquid '%s': depth min > max" % body.material_id)
		if not (_is_chance(body.chance) and _is_chance(body.surface_chance)):
			errors.append(at + "liquid '%s': chance must be 0 to 1" % body.material_id)
	for stratum in entry.strata:
		if material(stratum.material_id) == null:
			errors.append(at + "stratum of unknown material '%s'" % stratum.material_id)
		if stratum.min_cells < 0 or stratum.min_cells > stratum.max_cells:
			errors.append(
				(
					at
					+ (
						"stratum '%s': min_cells %d > max_cells %d"
						% [stratum.material_id, stratum.min_cells, stratum.max_cells]
					)
				)
			)
	return errors


func _validate_transitions(entry) -> PackedStringArray:
	var errors := PackedStringArray()
	for transition in entry.heat_transitions:
		var at := "'%s' %s" % [entry.id, transition.describe()]
		if transition.becomes.is_empty() and transition.gains_trait.is_empty():
			errors.append(at + ": becomes nothing and gains no trait")
		elif transition.becomes and material(transition.becomes) == null:
			errors.append(at + ": becomes unknown '%s'" % transition.becomes)
	return errors


static func _is_chance(value: float) -> bool:
	return value >= 0.0 and value <= 1.0


static func _validate_ids(entries: Array, kind: String) -> PackedStringArray:
	var errors := PackedStringArray()
	var seen := {}
	for entry in entries:
		if entry.id.is_empty():
			errors.append("%s id must not be empty" % kind)
		elif seen.has(entry.id):
			errors.append("duplicate %s id '%s'" % [kind, entry.id])
		seen[entry.id] = true
	return errors
