class_name RoadSegmentDef
extends Resource
## A road between two neighbouring cells (8-neighbours, so diagonals too), per
## specs/19-map-layout-and-objectives.md. Only the cells a road joins are connected;
## roads never auto-join. A road lowers movement cost (TerrainLibraryDef.
## road_move_multiplier); it isn't topology - links are.

@export var a: Vector2i = Vector2i.ZERO
@export var b: Vector2i = Vector2i.ZERO
