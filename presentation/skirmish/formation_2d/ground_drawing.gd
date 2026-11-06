class_name GroundDrawing
extends RefCounted
## How the 2D field draws what doesn't change while it plays (FieldState.ground and routes,
## read at open): the grass, the painted areas - the wood, a stream too deep to wade or its
## ford, and the hill, lighter a ring higher - and each route, a line in its corridor.


## Draws `ground` (the field's size, then its areas) and `routes` on `canvas`, a cell
## `cell_px` pixels from `origin`, in `colours` (FormationFieldScene.COLOURS).
static func draw(
	canvas: CanvasItem, ground: Array, routes: Dictionary, at: Array, colours: Dictionary
) -> void:
	var origin: Vector2 = at[0]
	var cell_px: float = at[1]
	canvas.draw_rect(Rect2(origin, Vector2(ground[0]) * cell_px), colours["grass"])
	for area in ground.slice(1):
		var cells := Rect2(area[0])
		var drawn := Rect2(origin + cells.position * cell_px, cells.size * cell_px)
		canvas.draw_rect(drawn, _colour(colours, area[1], area[2]))
	for key in routes:
		_route(canvas, routes[key], at, [colours[key]])


## A painted area's colour: the wood, a stream too deep to wade or its ford, or the hill,
## lighter a ring higher.
static func _colour(colours: Dictionary, kind: String, value: float) -> Color:
	match kind:
		"wood":
			return colours["wood"]
		"depth":
			return colours["stream"] if value > 1.0 else colours["ford"]
	return colours["hill"].lightened((value - 1.0) * 0.05)


## `route`: [its points, in cells, its half width] (FieldState.routes); `colour`: [its colour].
static func _route(canvas: CanvasItem, route: Array, at: Array, colour: Array) -> void:
	var points: PackedVector2Array = route[0]
	var drawn := PackedVector2Array()
	for point in points:
		drawn.append(at[0] + point * at[1])
	var corridor: Color = colour[0]
	corridor.a = 0.12
	canvas.draw_polyline(drawn, corridor, route[1] * 2.0 * at[1])
	canvas.draw_polyline(drawn, colour[0], 2.0)
