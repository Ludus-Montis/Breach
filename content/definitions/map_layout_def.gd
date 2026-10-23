class_name MapLayoutDef
extends Resource
## A map's spatial layer, per specs/19-map-layout-and-objectives.md: the designer grid,
## its explicitly authored tiles, roads and each link's route geometry. Terrain and
## feature ids resolve against terrain_library - the one shared library the crate loaded
## (content/terrain/terrain_library.json, Decision 32), never a per-map copy. Not saved
## with the map; a test may set a library of its own.

const TerrainLibraryDef = preload("res://content/definitions/terrain_library_def.gd")
const TileDef = preload("res://content/definitions/tile_def.gd")
const RoadSegmentDef = preload("res://content/definitions/road_segment_def.gd")
const RouteDef = preload("res://content/definitions/route_def.gd")
const ChannelDef = preload("res://content/definitions/channel_def.gd")

## Cells along a tile's edge: a tile is 64 x 64 cells, about 109 m (Decision 68).
const CELLS_PER_TILE := 64

@export var cols: int = 0
@export var rows: int = 0
## Pixels per cell; node positions are cell centres at this size.
@export var cell_size: int = 64
@export var default_terrain_id: String = ""
@export var tiles: Array[TileDef] = []
@export var roads: Array[RoadSegmentDef] = []
@export var routes: Array[RouteDef] = []
## Ground above this height (cells) is taken to continue, impassable and unsimulated
## (Decision 56); 64 = 4 tiles.
@export var ceiling: int = 64
## Seeds relief (and later strata and liquid bodies), so a map's ground is reproducible
## (Decision 59).
@export var seed: int = 0
## Rivers, ditches, moats and banks carved along tiles (Decision 59).
@export var channels: Array[ChannelDef] = []
## The one shared library (Decision 32); not saved with the map.
var terrain_library: TerrainLibraryDef = CrateContent.terrain_library()


func tile_at(cell: Vector2i) -> TileDef:
	for tile in tiles:
		if tile.cell == cell:
			return tile
	return null


func terrain_id_at(cell: Vector2i) -> String:
	var tile := tile_at(cell)
	return tile.terrain_id if tile != null and tile.terrain_id else default_terrain_id


func contains(cell: Vector2i) -> bool:
	return cell.x >= 0 and cell.y >= 0 and cell.x < cols and cell.y < rows


## A tile's ground height in cells: its own elevation, else its terrain's default.
func elevation_at(cell: Vector2i) -> int:
	var tile := tile_at(cell)
	if tile != null and tile.elevation != TileDef.DEFAULT:
		return tile.elevation
	var terrain = terrain_library.terrain(terrain_id_at(cell)) if terrain_library else null
	return terrain.default_elevation if terrain else 0


func validate(node_ids: Array) -> PackedStringArray:
	var errors := PackedStringArray()
	if ceiling <= 0:
		errors.append("layout ceiling must be > 0 cells, got %d" % ceiling)
	if cols <= 0 or rows <= 0:
		errors.append("layout cols and rows must be > 0, got %dx%d" % [cols, rows])
	if terrain_library == null:
		errors.append("layout has no terrain_library")
		return errors
	var default_terrain = terrain_library.terrain(default_terrain_id)
	if default_terrain == null or not default_terrain.can_be_base:
		errors.append(
			(
				"default_terrain_id '%s' must be a can_be_base terrain in the library"
				% default_terrain_id
			)
		)
	errors.append_array(_validate_tiles(node_ids))
	errors.append_array(_validate_roads())
	errors.append_array(_validate_routes(node_ids))
	for channel in channels:
		errors.append_array(channel.validate(cols, rows, terrain_library))
	return errors


func _validate_tiles(node_ids: Array) -> PackedStringArray:
	var errors := PackedStringArray()
	var seen := {}
	for tile in tiles:
		if not contains(tile.cell):
			errors.append("tile %s is outside the %dx%d grid" % [tile.cell, cols, rows])
		if seen.has(tile.cell):
			errors.append("duplicate tile at %s" % tile.cell)
		seen[tile.cell] = true
		errors.append_array(tile.validate())
		errors.append_array(_validate_tile_references(tile, node_ids))
	return errors


func _validate_tile_references(tile: TileDef, node_ids: Array) -> PackedStringArray:
	var errors := PackedStringArray()
	if tile.terrain_id and terrain_library.terrain(tile.terrain_id) == null:
		errors.append("tile %s: unknown terrain '%s'" % [tile.cell, tile.terrain_id])
	if tile.feature_id and terrain_library.feature(tile.feature_id) == null:
		errors.append("tile %s: unknown feature '%s'" % [tile.cell, tile.feature_id])
	if tile.bridge != null:
		var terrain = terrain_library.terrain(terrain_id_at(tile.cell))
		if terrain != null and not terrain.needs_bridge:
			errors.append("bridge at %s: '%s' doesn't need a bridge" % [tile.cell, terrain.id])
		var controller := tile.bridge.drawbridge_node_id
		if controller and not node_ids.has(controller):
			errors.append("bridge at %s: unknown drawbridge_node_id '%s'" % [tile.cell, controller])
	return errors


func _validate_roads() -> PackedStringArray:
	var errors := PackedStringArray()
	var seen := {}
	for road in roads:
		var label := "%s-%s" % [road.a, road.b]
		if not contains(road.a) or not contains(road.b):
			errors.append("road %s leaves the grid" % label)
		if not road.is_between_neighbours():
			errors.append("road %s must join neighbouring cells" % label)
		if seen.has(road.key()):
			errors.append("duplicate road %s" % label)
		seen[road.key()] = true
	return errors


func _validate_routes(node_ids: Array) -> PackedStringArray:
	var errors := PackedStringArray()
	for route in routes:
		for node_id in [route.node_a_id, route.node_b_id]:
			if not node_ids.has(node_id):
				errors.append("route %s: unknown node '%s'" % [route.label(), node_id])
		if route.cells.any(func(c): return not contains(c)):
			errors.append("route %s leaves the grid" % route.label())
		var gap := route.gap()
		if not gap.is_empty():
			errors.append("route %s has a gap between %s and %s" % [route.label(), gap[0], gap[1]])
	return errors
