class_name SubnodeDef
extends Resource
## One of a node's objectives (Decision 72, specs/26-subnodes.md): units take it by
## holding its capture area with no enemy there, and its holder then controls its zone of
## influence. Cells are columns in the node's local cells (Decision 68), all levels. A node
## is controlled only when one side holds every subnode; capture isn't simulated yet.

## Unique within its node.
@export var id: String = ""
@export var subnode_type: String = "OBJECTIVE"
## The marker cell the subnode was placed at.
@export var at: Vector2i = Vector2i.ZERO
## Where units must stand to take it; inside the zone.
@export var capture_cells: Array[Vector2i] = []
## The cells its holder controls (building, benefit, buildings within).
@export var zone_cells: Array[Vector2i] = []
