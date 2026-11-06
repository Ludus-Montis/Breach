class_name FormationFieldScene
extends Node2D
## The 2D formation feel test (Decision 86, spec 27 rounds 1 and 2): a top-down view of the
## 2D field - the kingdom's line across the middle, the player's routes A and B with
## their corridors, and every unit drawn as its body (Decision 106), a tick at its front,
## eased between ticks. Send a route's wave, let it depart when full, send both timed to
## arrive together, or have B wait in the wood (its detection range ringed) until it sees
## A engage (Decision 87). In a fight units seek contact and face their own foes (Decision
## 88); "Line has a captain" restarts with a led line that turns to meet a flank; Reset
## starts afresh under a battle seed (Decision 93). Every order is recorded with its tick
## (FormationRecord, Decision 115), for copying out; a record pasted back replays live.
## Controls: FormationFieldHud.
## Engine glue: the field is the crate's (`FieldState`, spec 39 step 9d-i), played on a
## worker (step 10d-ii). Each frame owes it the clock's ticks and takes what it has played,
## never waiting; the scene draws the newest tick, and the HUD says when the simulation is
## behind (more time due than the worker is owed, so some is dropped).
##
##   godot --path . res://presentation/skirmish/formation_2d/formation_field.tscn

const FallenDrawing = preload("res://presentation/skirmish/formation_2d/fallen_drawing.gd")
const GroundDrawing = preload("res://presentation/skirmish/formation_2d/ground_drawing.gd")
const FormationFieldHud = preload("res://presentation/skirmish/formation_2d/formation_field_hud.gd")

const CELL_PX := 8.0
const ORIGIN := Vector2(16, 56)
const FLASH_SECONDS := 0.3
const BEHIND_SECONDS := 1.0  # how long "simulation behind" shows after time is dropped
const COLOURS := {
	"grass": Color(0.33, 0.45, 0.25),
	"wood": Color(0.16, 0.3, 0.14),
	"stream": Color(0.18, 0.35, 0.65),
	"ford": Color(0.45, 0.62, 0.8),
	"hill": Color(0.5, 0.45, 0.3),
	"A": Color(0.95, 0.85, 0.3),
	"B": Color(0.95, 0.55, 0.2),
	"C": Color(0.85, 0.4, 0.7),
	"player": Color(0.6, 0.3, 0.75),
	"the_kingdom": Color(0.3, 0.5, 0.9),
	"flash": Color(1, 0.2, 0.2),
	"leader": Color(0.35, 0.1, 0.45),
	"staging": Color(1, 1, 1, 0.8),
	"sight": Color(1, 1, 1, 0.25),
}

## The crate's SkirmishClock (spec 39 step 9c).
var _clock: Object = NativeLibrary.made("SkirmishClock")
var _field: Object  # the crate's FieldState
var _squads: Array = []  # the field's squads as read after the latest tick (FieldState.squads)
var _ground: Array = []  # the field's size, then its areas (FieldState.ground)
var _routes := {}  # the routes by key (FieldState.routes)
var _previous := {}  # unit id -> position (cells) at the previous tick
var _current := {}  # unit id -> position (cells) at the latest tick
var _flashes := {}  # unit id -> seconds left
var _hud: FormationFieldHud
var _battle_seed := 0
var _squads_before := {}  # squad id -> its front's position at the previous tick
var _squads_now := {}
var _behind := 0.0  # seconds "simulation behind" still shows
var _fraction := 1.0  # how far this frame draws between the last two ticks


func field() -> Object:
	return _field


func clock() -> Object:
	return _clock


## Runs `count` ticks, and every order given before, waiting for them (for tests; the frame
## loop never waits); their events, in order.
func run_ticks(count: int) -> Array:
	var before: int = _field.tick_number()
	var events: Array = _field.run_ticks(count)
	_took(events, before)
	return events


## `events`, of the ticks played since tick `before`: flank hits flash; a new tick is drawn.
func _took(events: Array, before: int) -> void:
	for event in events:
		if event["type"] == "hit" and event["flank"]:
			_flashes[event["target"]] = FLASH_SECONDS
	if _field.tick_number() != before:
		_snapshot()


func _ready() -> void:
	restart(false, randi() % 1000000)
	_hud = FormationFieldHud.new()
	add_child(_hud)
	_hud.build(self)
	var camera := Camera2D.new()
	camera.position = ORIGIN + Vector2(_ground[0]) * CELL_PX * 0.5 - Vector2(0, 40)
	add_child(camera)


## A fresh field, its line led by a captain or not, its battle under `battle_seed`
## (Decision 93: the same seed and orders replay a battle). `queued` (commands,
## FieldState.read_record) are given at their ticks as the clock runs: a replay.
func restart(captained: bool, battle_seed: int, queued: Array = []) -> void:
	_battle_seed = battle_seed
	_field = NativeLibrary.made("FieldState")  # the old one's tick finishes and is dropped
	_field.open(CrateContent.store(), battle_seed, captained, SimBuild.id())
	var commands: Array[Dictionary] = []
	commands.assign(queued)
	_field.replay(commands)  # each given when its tick comes
	_ground = _field.ground()
	_routes = _field.routes()
	_previous = {}
	_current = {}
	_snapshot()


## A frame of `delta` seconds: the clock's ticks owed to the worker and what it has played
## taken, never waiting; then drawn.
func _process(delta: float) -> void:
	var due: int = _clock.advance(delta)
	_field.pause(_clock.is_paused())
	_behind = BEHIND_SECONDS if _field.owe(due) else maxf(_behind - delta, 0.0)
	var before: int = _field.tick_number()
	_took(_field.take(), before)
	var drawing: bool = not _clock.is_paused() and _field.caught_up()
	_fraction = _clock.fraction() if drawing else 1.0  # else hold at the newest
	for unit_id in _flashes.keys():
		_flashes[unit_id] -= delta
		if _flashes[unit_id] <= 0.0:
			_flashes.erase(unit_id)
	if _hud != null:
		_hud.show_status(_field, _clock.is_paused(), _battle_seed, _behind > 0.0)
	queue_redraw()


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode == KEY_SPACE:
		toggle_pause()


func _draw() -> void:
	GroundDrawing.draw(self, _ground, _routes, [ORIGIN, CELL_PX], COLOURS)
	var fraction := _fraction
	_draw_staging()
	var at: PackedVector2Array = _field.positions()  # a frame's
	for squad in _squads:
		for unit in squad["units"]:
			if not unit["alive"] and unit["state"] != "CARRIED":
				var radius := _radius(unit)
				var centre: Vector2 = ORIGIN + at[unit["slot"]] * CELL_PX
				FallenDrawing.draw(self, unit["state"], centre, radius, COLOURS[squad["faction"]])
	var facings: PackedVector2Array = _field.facings()
	var ranks: PackedInt32Array = _field.ranks()
	for squad in _squads:
		for unit in squad["units"]:
			if unit["alive"]:
				var now := [at[unit["slot"]], facings[unit["slot"]], ranks[unit["slot"]]]
				_draw_unit(squad, unit, fraction, now)
		_draw_morale(squad)
		_draw_pursuit(squad)
	for squad in _squads:
		for unit in squad["units"]:
			if unit["state"] == "CARRIED":  # on top of its bearer
				var centre: Vector2 = ORIGIN + at[unit["slot"]] * CELL_PX
				var colour: Color = COLOURS[squad["faction"]]
				FallenDrawing.draw(self, "CARRIED", centre, CELL_PX * 0.5, colour)


static func _radius(unit: Dictionary) -> float:
	return minf(unit["footprint_width"], unit["footprint_depth"]) / 2.0 * CELL_PX - 0.5


## `now`: [where the unit stands, the way it faces, its rank], from this frame's columns.
func _draw_unit(squad: Dictionary, unit: Dictionary, fraction: float, now: Array) -> void:
	var stands: Vector2 = now[0]
	var before: Vector2 = _previous.get(unit["id"], stands)
	var after: Vector2 = _current.get(unit["id"], stands)
	var centre := ORIGIN + before.lerp(after, fraction) * CELL_PX
	var radius := _radius(unit)
	var flashed := _flashes.has(unit["id"])
	var colour: Color = COLOURS["flash"] if flashed else COLOURS[squad["faction"]]
	if unit["leadership"] > 0 and not flashed:
		colour = COLOURS["leader"]
	if squad["state"] == "ROUTING":
		colour.a = 0.45  # routers flee one by one
	draw_circle(centre, radius, colour)  # its body (Decision 106)
	if now[2] == 0 or Array(squad["loose"]).has(unit["id"]):
		var front: Vector2 = centre + now[1] * radius
		draw_circle(front, 1.5, Color.WHITE)


## The staging point in the wood, and a ring of detection round any wave waiting there.
func _draw_staging() -> void:
	var staging: Variant = _field.staging_point()
	if staging == null:
		return
	var point: Vector2 = ORIGIN + staging * CELL_PX
	draw_colored_polygon(
		PackedVector2Array(
			[
				point + Vector2(0, -5),
				point + Vector2(5, 0),
				point + Vector2(0, 5),
				point + Vector2(-5, 0)
			]
		),
		COLOURS["staging"]
	)
	for squad in _squads:
		var living: Array = squad["units"].filter(func(u): return u["alive"])
		if not squad["staged"] or living.is_empty():
			continue
		var reach: float = living.map(func(u): return u["detection"]).max()
		var centre: Vector2 = ORIGIN + squad["position"] * CELL_PX
		draw_arc(centre, reach * CELL_PX, 0.0, TAU, 64, COLOURS["sight"], 1.5)


## A pursuing squad's post and how far its leash lets it go (Decision 107): a cross at the
## post, ringed at the leash (it pursues along whatever route its quarry flees by).
func _draw_pursuit(squad: Dictionary) -> void:
	if squad["pursuit"] == null:
		return
	var post: Vector2 = ORIGIN + Vector2(squad["pursuit"]["post_at"]) * CELL_PX
	var leash: float = squad["pursuit"]["leash"]
	var colour: Color = COLOURS[squad["faction"]]
	draw_line(post - Vector2(4, 4), post + Vector2(4, 4), colour, 2.0)
	draw_line(post - Vector2(4, -4), post + Vector2(4, -4), colour, 2.0)
	if not is_inf(leash):
		draw_arc(post, leash * CELL_PX, 0.0, TAU, 96, colour, 1.0)


## A bar over the squad's front: its morale, coloured by band (Decision 82).
func _draw_morale(squad: Dictionary) -> void:
	var standing: bool = squad["units"].any(func(u): return u["alive"])
	if not standing or squad["state"] == "ROUTING":
		return
	var band_colours := [Color(0.3, 0.9, 0.3), Color(0.95, 0.85, 0.2), Color(1, 0.5, 0.1)]
	var band: int = squad["band"]
	var colour: Color = band_colours[band] if band < band_colours.size() else Color.RED
	var fraction := _fraction
	var position: Vector2 = squad["position"]
	var before: Vector2 = _squads_before.get(squad["id"], position)
	var front: Vector2 = before.lerp(_squads_now.get(squad["id"], position), fraction)
	var top := ORIGIN + front * CELL_PX + Vector2(-12, -6 * CELL_PX)  # eased, as its units
	draw_rect(Rect2(top, Vector2(24, 3)), Color(0, 0, 0, 0.5))
	draw_rect(Rect2(top, Vector2(24 * squad["morale"] / 100.0, 3)), colour)


func _snapshot() -> void:
	_previous = _current
	_current = {}
	_squads_before = _squads_now
	_squads_now = {}
	_squads = _field.squads()
	for squad in _squads:
		_squads_now[squad["id"]] = squad["position"]
	var at: PackedVector2Array = _field.positions()
	for squad in _squads:
		for unit in squad["units"]:
			_current[unit["id"]] = at[unit["slot"]]


## Gives an order - a command (FieldState.read_record) or the feel test's words for one -
## at the next boundary between ticks; the field records it with that tick if it takes it.
func act(order: Variant) -> void:
	if order is Dictionary:
		_field.give(order)
	else:
		_field.act(order)


## The session so far, to copy out: a replay of it gives the same battle.
func action_log() -> String:
	return _field.record()


func toggle_pause() -> void:
	if _clock.is_paused():
		_clock.resume()
	else:
		_clock.pause()
