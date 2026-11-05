class_name WavePainter
extends Control
## Paints a lane's wave template (Decisions 42-43, specs/22-formation-feel-test.md). The
## grid faces the direction of travel: the front rank is the right-hand column, formation
## columns run top to bottom, and rows past the lane's width are shaded. Template places
## are outlined; built ones are filled, and each place a builder is working toward fills
## with that build's progress (Decision 45).
## Pressing an empty cell paints with the brush, pressing a filled one erases it (a drag
## keeps doing whichever the press started); right click always erases. The scene applies
## each change at once (built units fold in, leftovers are banked).

signal painted(cell: Vector2i)
signal erased(cell: Vector2i)

const CELL := 20.0
const TOP := 14.0
const COLUMNS := 8
const RANKS := 4

## Formation columns this lane allows (the rest are shaded).
var lane_width := COLUMNS
## [[rank, column, depth, width, filled, preferred position, kind, progress], ...] from
## FormationSkirmishReadout.lane().
var places := []
## Fill colour by the unit's preferred band (the unit's `preferred_position`): front, mid, back.
var band_colors := [Color("#b3761d"), Color("#8a6fb3"), Color("#3f8f86")]

var _last_cell := Vector2i(-1, -1)
var _erasing := false


func _ready() -> void:
	custom_minimum_size = Vector2(RANKS * CELL + 60.0, TOP + COLUMNS * CELL + 2.0)
	mouse_filter = Control.MOUSE_FILTER_STOP


## The (rank, formation column) under a point.
func cell_at(point: Vector2) -> Vector2i:
	return Vector2i(RANKS - 1 - floori(point.x / CELL), floori((point.y - TOP) / CELL))


## True if a template place covers the cell.
func covers(cell: Vector2i) -> bool:
	return places.any(
		func(p):
			return (
				p[0] <= cell.x and cell.x < p[0] + p[2] and p[1] <= cell.y and cell.y < p[1] + p[3]
			)
	)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		_last_cell = Vector2i(-1, -1)
		var right: bool = event.button_index == MOUSE_BUTTON_RIGHT
		_erasing = right or covers(cell_at(event.position))
		_act(event.position)
	elif (
		event is InputEventMouseMotion
		and event.button_mask & (MOUSE_BUTTON_MASK_LEFT | MOUSE_BUTTON_MASK_RIGHT)
	):
		_act(event.position)


func _act(point: Vector2) -> void:
	var cell := cell_at(point)
	if cell == _last_cell or cell.x < 0 or cell.y < 0 or cell.x >= RANKS or cell.y >= lane_width:
		return
	_last_cell = cell
	if _erasing:
		erased.emit(cell)
	else:
		painted.emit(cell)
	accept_event()


func _draw() -> void:
	draw_string(
		ThemeDB.fallback_font,
		Vector2(0, TOP - 3.0),
		"back          front ▶",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		11,
		Color(1, 1, 1, 0.7)
	)
	for rank in range(RANKS):
		for column in range(COLUMNS):
			var usable := column < lane_width
			var shade := Color(1, 1, 1, 0.12) if usable else Color(0, 0, 0, 0.35)
			draw_rect(_rect(rank, column, 1, 1).grow(-1.0), shade)
	for place in places:
		var body := _rect(place[0], place[1], place[2], place[3]).grow(-2.0)
		var progress: float = place[7] if place.size() > 7 else 0.0
		var fill_color: Color = band_colors[place[5] if place.size() > 5 else 0]
		if place[4]:
			draw_rect(body, fill_color)
		elif progress > 0.0:  # builds from the back of the body towards the front
			draw_rect(
				Rect2(body.position, Vector2(body.size.x * progress, body.size.y)),
				fill_color.darkened(0.3)
			)
		draw_rect(body, Color.WHITE, false, 1.5)


## The display rect of a footprint: its front rank is its right-hand edge.
func _rect(rank: int, column: int, depth: int, width: int) -> Rect2:
	var left := (RANKS - rank - depth) * CELL
	return Rect2(Vector2(left, TOP + column * CELL), Vector2(depth * CELL, width * CELL))
