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
