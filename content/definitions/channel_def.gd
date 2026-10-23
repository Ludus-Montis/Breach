class_name ChannelDef
extends Resource
## A carved feature along a path of tiles (Decision 59): a river or stream, a ditch or a
## moat - or, with a negative depth, a bank or mound. Its centre line runs through the
## tile centres; it is `width` cells across (with a one-cell sloped bank), `depth` cells
## deep, and can hold a flowing material (water, lava) `liquid_depth` cells deep.

## Tiles (col, row) in order; each next to the one before (diagonals too).
@export var tiles: Array[Vector2i] = []
@export var width: int = 4
## Cells cut below the ground; negative raises a bank instead.
@export var depth: int = 2
## A material that flows, or "" for a dry ditch.
@export var liquid_id: String = ""
@export var liquid_depth: int = 0
