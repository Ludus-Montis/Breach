class_name RouteDef
extends Resource
## The initial path of one link between two nodes, per specs/19-map-layout-and-
## objectives.md and Decision 26: links are the authored routes, and the designer
## pathfinds each one across terrain cost. cells runs from node_a's cell to node_b's.

@export var node_a_id: String = ""
@export var node_b_id: String = ""
@export var cells: Array[Vector2i] = []
## The designer's pathfinding cost for these cells.
@export var cost: float = 0.0


func label() -> String:
	return "%s-%s" % [node_a_id, node_b_id]
