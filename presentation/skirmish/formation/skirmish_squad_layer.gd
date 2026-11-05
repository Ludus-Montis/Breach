class_name SkirmishSquadLayer
extends Node2D
## Draws the formation feel test's squads, per specs/22-formation-feel-test.md: every unit
## as a footprint-sized block (width across the route, depth along it), each lane's
## formation laid across its route (SkirmishRoute.normal_at), positions interpolated
## between ticks, a flash on flank hits, a dot on ranged units and a line for each spit
## (Decision 46), and a ring around the selected squad. Read-only: it draws each lane's
## squads as the crate's BattleState gives them after a tick (spec 39 step 9d-ii).

const SkirmishRoute = preload("res://presentation/skirmish/skirmish_route.gd")
const TickInterpolation = preload("res://presentation/tick_interpolation.gd")

## A rank's depth along the lane, in tiles (the crate's rank depth: a cell).
const RANK_DEPTH := 1.0 / 64.0
## Pixels per formation column across the route, and per rank along it (Decision 48: a
## fifth of the size they were).
const COLUMN_PX := 17.0 * 0.2 * 0.25  # a quarter again with 64-cell tiles (Decision 68)
const RANK_PX := RANK_DEPTH * 64.0 * 1.3
const FLASH_SECONDS := 0.35
## One formation cell's smaller side on screen: outlines, bars and markers scale with it,
## so they stay in proportion to the (small) units at any zoom.
const CELL_PX := minf(RANK_PX, COLUMN_PX) * 0.85

var cell_size := 64.0
var faction_colors := {"player": Color("#b3761d"), "the_kingdom": Color("#a3372a")}
var fraction := 1.0
## [lane key, squad id] of the selected squad, or [] for none.
var selected: Array = []
## The crate's BattleState the squads are read from.
var battle: Object

var _lanes := {}  # lane key -> its route's points
var _squads := {}  # lane key -> its squads (BattleState.squads) after the latest tick
## Keyed "lane:unit id" - unit ids are only unique within one lane's battle.
var _previous := {}  # "lane:id" -> distance at the previous tick
var _current := {}
var _flashes := {}  # "lane:id" -> seconds left
var _spits := []  # [lane key, shooter id, target id, seconds left]


func add_lane(lane_key: String, points: PackedVector2Array) -> void:
	_lanes[lane_key] = points
	_squads[lane_key] = []


## Call after each tick: the squads are read again and the latest distances become the
## interpolation target.
func snapshot() -> void:
	_previous = _current.duplicate()
	_current = {}
	for lane_key in _lanes:
		_squads[lane_key] = battle.squads(lane_key) if battle != null else []
		for squad in _squads[lane_key]:
			for unit in squad["units"]:
				var unit_key := _key(lane_key, unit["id"])
				_current[unit_key] = unit["distance"]
				if not _previous.has(unit_key):
					_previous[unit_key] = unit["distance"]


## A lane's squads as read after the latest tick.
func squads(lane_key: String) -> Array:
	return _squads.get(lane_key, [])


func flash(lane_key: String, unit_id: int) -> void:
	_flashes[_key(lane_key, unit_id)] = FLASH_SECONDS


## A ranged strike, drawn briefly as a line from the shooter to its target.
func spit(lane_key: String, shooter_id: int, target_id: int) -> void:
	_spits.append([lane_key, shooter_id, target_id, FLASH_SECONDS])


## [lane key, squad id] of the squad whose unit is under a point, or [].
func squad_at(point: Vector2) -> Array:
	for lane_key in _lanes:
		for each in squads(lane_key):
			for unit in each["units"]:
				var reach: float = COLUMN_PX * unit["footprint_width"]
				if _centre(lane_key, each, unit).distance_to(point) <= reach:
					return [lane_key, each["id"]]
	return []


## Selects the next of a faction's squads across the lanes (Tab); none if it has none.
func cycle_selection(faction_id: String) -> void:
	var options := []
	for lane_key in _lanes:
		for each in squads(lane_key):
			if each["faction"] == faction_id and each["state"] != "DESTROYED":
				options.append([lane_key, each["id"]])
	var at := options.find(selected)
	selected = [] if options.is_empty() else options[(at + 1) % options.size()]


func _process(delta: float) -> void:
	for unit_key in _flashes.keys():
		_flashes[unit_key] -= delta
		if _flashes[unit_key] <= 0.0:
			_flashes.erase(unit_key)
	for shot in _spits:
		shot[3] -= delta
	_spits = _spits.filter(func(shot): return shot[3] > 0.0)
	queue_redraw()


func _draw() -> void:
	for lane_key in _lanes:
		for each in squads(lane_key):
			var chosen: bool = selected == [lane_key, each["id"]]
			for unit in each["units"]:
				_draw_unit(lane_key, each, unit, chosen)
	for shot in _spits:
		var ends := [_find(shot[0], shot[1]), _find(shot[0], shot[2])]
		if not ends.has(null):
			var tint := Color(0.55, 0.95, 0.6, shot[3] / FLASH_SECONDS)
			draw_line(
				_centre(shot[0], ends[0][0], ends[0][1]),
				_centre(shot[0], ends[1][0], ends[1][1]),
				tint,
				CELL_PX * 0.2
			)


func _draw_unit(lane_key: String, squad: Dictionary, unit: Dictionary, chosen: bool) -> void:
	var points: PackedVector2Array = _lanes[lane_key]
	var centre := _centre(lane_key, squad, unit)
	var along := SkirmishRoute.normal_at(points, _distance(lane_key, unit), cell_size).rotated(
		-PI / 2.0
	)
	var size := Vector2(
		unit["footprint_depth"] * RANK_PX * 0.85, unit["footprint_width"] * COLUMN_PX * 0.85
	)
	var body := Rect2(-size / 2.0, size)
	var color: Color = faction_colors.get(squad["faction"], Color.GRAY)
	if squad["state"] == "ARRIVED":
		color.a = 0.45
	draw_set_transform(centre, along.angle())
	if chosen:
		draw_rect(body.grow(CELL_PX * 0.4), Color.WHITE, false, CELL_PX * 0.25)
	draw_rect(body, color)
	draw_rect(body, Color("#211d15"), false, CELL_PX * 0.12)
	var share := clampf(float(unit["hp"]) / float(maxi(unit["max_hp"], 1)), 0.0, 1.0)
	var bar := Vector2(body.size.x * share, CELL_PX * 0.22)
	draw_rect(Rect2(body.position, bar), Color("#7fbf88"))
	var unit_key := _key(lane_key, unit["id"])
	if _flashes.has(unit_key):
		draw_rect(
			body.grow(CELL_PX * 0.3),
			Color(1.0, 0.95, 0.4, _flashes[unit_key] / FLASH_SECONDS),
			false,
			CELL_PX * 0.3
		)
	if unit["ranged"]:
		draw_circle(Vector2.ZERO, CELL_PX * 0.22, Color("#d8f0c0"))
	if unit["target"] != 0:
		draw_circle(Vector2(size.x * 0.5 * squad["direction"], 0), CELL_PX * 0.15, Color.WHITE)
	draw_set_transform(Vector2.ZERO, 0.0)


func _distance(lane_key: String, unit: Dictionary) -> float:
	var unit_key := _key(lane_key, unit["id"])
	var current: float = _current.get(unit_key, unit["distance"])
	var previous: float = _previous.get(unit_key, current)
	return TickInterpolation.interpolate_position(previous, current, fraction)


## A unit's drawn centre: along the route at its (interpolated) distance, back by half its
## depth, and across the route at the centre of its lateral span (a swap's move added).
func _centre(lane_key: String, squad: Dictionary, unit: Dictionary) -> Vector2:
	var points: PackedVector2Array = _lanes[lane_key]
	var depth_back: float = (unit["footprint_depth"] - 1) * RANK_DEPTH * 0.5
	var distance: float = _distance(lane_key, unit) - squad["direction"] * depth_back
	var lateral: float = unit["lateral"] * COLUMN_PX
	return (
		SkirmishRoute.point_at(points, distance, cell_size)
		+ SkirmishRoute.normal_at(points, distance, cell_size) * lateral
	)


## [squad, unit] for a living unit id in a lane, or null.
func _find(lane_key: String, unit_id: int) -> Variant:
	for each in squads(lane_key):
		for unit in each["units"]:
			if unit["id"] == unit_id:
				return [each, unit]
	return null


func _key(lane_key: String, unit_id: int) -> String:
	return "%s:%d" % [lane_key, unit_id]
