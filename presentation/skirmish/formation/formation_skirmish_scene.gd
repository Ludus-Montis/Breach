class_name FormationSkirmishScene
extends Node2D
## The formation feel test (specs/22-formation-feel-test.md, Decisions 39-45): two lanes
## from P in one FormationBattle (domain builders and reserve, slot pool, reshapes with
## fold + bank), stepped by one SkirmishClock; the player's saved presets, kingdom militia
## lines and wave-level orders. Engine glue - the rules live in sim/skirmish/formation/.
##
##   godot --path . res://presentation/skirmish/formation/formation_skirmish.tscn

const SkirmishClock = preload("res://sim/skirmish/skirmish_clock.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SkirmishSlotPool = preload("res://sim/skirmish/formation/skirmish_slot_pool.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationBattle = preload("res://sim/skirmish/formation/formation_battle.gd")
const FormationProduction = preload("res://sim/skirmish/formation/formation_production.gd")
const WavePresets = preload("res://sim/skirmish/formation/wave_presets.gd")
const WavePresetStore = preload("res://presentation/skirmish/formation/wave_preset_store.gd")
const SkirmishRoute = preload("res://presentation/skirmish/skirmish_route.gd")
const SkirmishCamera = preload("res://presentation/skirmish/skirmish_camera.gd")
const FormationSkirmishHud = preload(
	"res://presentation/skirmish/formation/formation_skirmish_hud.gd"
)
const MapView = preload("res://presentation/map_view.gd")
const FormationSkirmishReadout = preload(
	"res://presentation/skirmish/formation/formation_skirmish_readout.gd"
)

const POOL_TOTAL := 8
## Each lane's maximum frontline width (feel-test config; maps author this later).
const LANE_WIDTHS := {"c": 8, "k": 8}
const START_SLOTS := {"c": 5, "k": 3}
const KINGDOM_LINE := 3
## The builder types' names and starting counts, in HUD order (Decision 45).
const BUILDER_NAMES := ["Grem", "Brute", "Spitter"]
const START_BUILDERS := [2, 1, 1]

## Decision 39: a full wave pauses the game (true) or only notifies (false).
var pause_on_wave_full := true
## Where the player's presets are kept; empty keeps them in memory only.
var presets_path := "user://formation_presets.json"

var _map: MapDef = CrateContent.map("skirmish_two_lanes")
## The units it fields (CrateContent); brushes in HUD order (hotkeys 1, 2, 3, E; null
## erases), the builder types in HUD order, and unit kinds by name, for saved presets.
var _grem := CrateContent.unit("grem")
var _brute := CrateContent.unit("grem_brute")
var _spitter := CrateContent.unit("grem_spitter")
var _militia := CrateContent.unit("kingdom_militia")
var _brushes: Array = [_grem, _brute, _spitter, null]
var _builder_types: Array = [_grem, _brute, _spitter]
var _kinds := {"grem": _grem, "brute": _brute, "spitter": _spitter}
var _clock := SkirmishClock.new()
var _battle: FormationBattle
var _kingdom_auto := false
var _paused_for_wave := false
var _hud: FormationSkirmishHud
var _brush := 0
var _presets := []  # WavePresets dictionaries, the player's own


func lane_keys() -> Array:
	return _battle.lane_keys()


func simulation(lane_key: String) -> FormationSimulation:
	return _battle.lane(lane_key).sim


func pool() -> SkirmishSlotPool:
	return _battle.pool


func clock() -> SkirmishClock:
	return _clock


func _ready() -> void:
	var view: MapView = $MapView
	view.map = _map
	_battle = FormationBattle.new(
		POOL_TOTAL, LANE_WIDTHS, _clock.tick_seconds, _militia, KINGDOM_LINE
	)
	for index in range(_builder_types.size()):
		_battle.player.set_builders(_builder_types[index], START_BUILDERS[index])
	_battle.player.prefer("")  # round robin by default (Decision 51)
	for route in _map.layout.routes:
		var points := view.model.layout_view.route_points(route.node_a_id, route.node_b_id)
		_add_lane(route.node_b_id, points)
	_presets = WavePresetStore.load_presets(presets_path)
	_hud = FormationSkirmishHud.new()
	add_child(_hud)
	_hud.build(lane_keys(), BUILDER_NAMES)
	_hud.set_tools(_brush, WavePresetStore.names(_presets))
	_connect_hud()
	var panel := FormationSkirmishHud.PANEL_WIDTH + 30.0
	SkirmishCamera.frame($Camera, $MapView.model.bounds, get_viewport_rect().size, panel)
	_refresh_hud()


func _add_lane(lane_key: String, points: PackedVector2Array) -> void:
	var length := SkirmishRoute.length_cells(points, _map.layout.cell_size)
	var start := WavePresets.line(_grem, START_SLOTS[lane_key], LANE_WIDTHS[lane_key])
	_battle.add_lane(lane_key, length, start)
	var lane := _battle.lane(lane_key)
	lane.brush = _grem
	lane.set_auto_departure(true)  # the defaults of Decision 51
	lane.sim.combat_width = LANE_WIDTHS[lane_key]
	$Squads.cell_size = _map.layout.cell_size
	$Squads.add_lane(lane_key, lane.sim, points)


func _process(delta: float) -> void:
	var due := _clock.advance(delta)
	for i in range(due):
		var events := _battle.step(_kingdom_auto)
		for key in events:
			handle_events(key, events[key])
		$Squads.snapshot()
		if _clock.is_paused():
			break
	$Squads.fraction = 1.0 if _clock.is_paused() else _clock.fraction()
	_refresh_hud()


## A tick's events for one lane: wave-full (Decision 39), flank flashes and the log.
func handle_events(lane_key: String, events: Array) -> void:
	for event in events:
		match event["type"]:
			"wave_full":
				if event["faction"] == "player":
					_on_wave_full(lane_key)
			"hit":
				if event["flank"]:
					$Squads.flash(lane_key, event["target"])
					_log(lane_key, event, "flank hit on #%d (%d)" % [event["target"], event["dmg"]])
			"spat":
				$Squads.spit(lane_key, event["unit"], event["target"])
			"engaged", "destroyed", "stepped_up", "arrived", "departed", "returned", "reinforced", "merged":
				_log(lane_key, event, event["type"].replace("_", " "))


## Sends the lane's wave; after a wave-full pause it also resumes (spec 21, playtest 1).
func send_wave(lane_key: String) -> void:
	_battle.lane(lane_key).production.send(simulation(lane_key))
	if _paused_for_wave:
		_paused_for_wave = false
		_clock.resume()


## Paints (or erases) one cell of a lane's wave; erasing frees slots for any lane at once
## (Decision 43), and displaced units go to the domain reserve (Decision 45).
func _paint_cell(lane_key: String, cell: Vector2i, erase: bool = false) -> void:
	_log_banked(lane_key, _battle.paint(lane_key, cell, erase))


func _save_preset(lane_key: String) -> void:
	var preset_name := "Preset %d" % (_presets.size() + 1)
	_presets.append(WavePresets.to_dict(_battle.lane(lane_key).template, preset_name, _kinds))
	WavePresetStore.save_presets(presets_path, _presets)
	if _hud != null:
		_hud.set_tools(_brush, WavePresetStore.names(_presets))
	_log(lane_key, {"faction": "player"}, "saved the shape as %s" % preset_name)


## Applies a saved preset, fitted to the lane's width and the slots it can reach.
func _apply_preset(lane_key: String, index: int) -> void:
	var allowance := _battle.allowance(lane_key)
	var fitted := WavePresets.from_dict(_presets[index], _kinds, LANE_WIDTHS[lane_key], allowance)
	_log_banked(lane_key, _battle.apply(lane_key, fitted))


func _log_banked(lane_key: String, banked: int) -> void:
	if banked > 0:
		var at := {"tick": simulation(lane_key).tick_number(), "faction": "player"}
		_log(lane_key, at, "reshaped: %d banked in the reserve" % banked)


## Adds or removes one of the domain's builders of a type (feel-test stand-in, Decision 45).
func _set_builders(type_index: int, delta: int) -> void:
	var count := _battle.player.builder_count(_builder_types[type_index])
	_battle.player.set_builders(_builder_types[type_index], maxi(count + delta, 0))


## Index < lane count: that lane first; the last option: round robin.
func _choose_distribution(index: int) -> void:
	_battle.player.prefer(lane_keys()[index] if index < lane_keys().size() else "")


## Only a lane that waits for Send pauses; one that departs on its own says so (Decision 51).
func _on_wave_full(lane_key: String) -> void:
	var departure := _battle.lane(lane_key).production.departure
	var waits := departure == FormationProduction.Departure.MANUAL
	var pausing := waits and pause_on_wave_full and not _clock.is_paused()
	if pausing:
		_clock.pause()
		_paused_for_wave = true
	var note := " - paused. Send it to go" if pausing else ""
	_banner("Lane %s: wave %s%s" % [lane_key, "ready" if waits else "departed", note])


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		match event.keycode:
			KEY_SPACE:
				_toggle_pause()
			KEY_S:
				send_wave("k" if event.shift_pressed else "c")
			KEY_A:
				_order(SkirmishUnit.Order.ADVANCE)
			KEY_H:
				_order(SkirmishUnit.Order.HOLD)
			KEY_R:
				_order(SkirmishUnit.Order.RETREAT)
			KEY_TAB:
				$Squads.cycle_selection("player")
			KEY_1, KEY_2, KEY_3, KEY_E:
				_choose_brush({KEY_1: 0, KEY_2: 1, KEY_3: 2, KEY_E: 3}[event.keycode])
	elif (
		event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	):
		$Squads.selected = $Squads.squad_at($Squads.get_local_mouse_position())


func _connect_hud() -> void:
	_hud.pause_toggled.connect(_toggle_pause)
	_hud.speed_chosen.connect(func(multiplier): _clock.speed_multiplier = multiplier)
	_hud.pause_on_full_toggled.connect(func(on): pause_on_wave_full = on)
	_hud.kingdom_auto_toggled.connect(func(on): _kingdom_auto = on)
	_hud.order_chosen.connect(_order)
	_hud.brush_chosen.connect(_choose_brush)
	_hud.builders_changed.connect(_set_builders)
	_hud.distribution_chosen.connect(_choose_distribution)
	_hud.preset_saved.connect(_save_preset)
	_hud.preset_applied.connect(_apply_preset)
	_hud.cell_painted.connect(func(key, cell): _paint_cell(key, cell))
	_hud.cell_erased.connect(func(key, cell): _paint_cell(key, cell, true))
	_hud.send_wave_pressed.connect(send_wave)
	_hud.auto_departure_toggled.connect(func(key, on): _battle.lane(key).set_auto_departure(on))
	_hud.auto_merge_toggled.connect(func(key, on): _battle.lane(key).production.auto_merge = on)
	_hud.spawn_kingdom_pressed.connect(
		func(key): _battle.lane(key).spawn_kingdom_line(_militia, KINGDOM_LINE)
	)


func _choose_brush(brush: int) -> void:
	_brush = brush
	for key in lane_keys():
		_battle.lane(key).brush = _brushes[brush]
	if _hud != null:
		_hud.set_tools(_brush, WavePresetStore.names(_presets))


func _toggle_pause() -> void:
	_paused_for_wave = false
	if _clock.is_paused():
		_clock.resume()
	else:
		_clock.pause()


func _order(new_order: int) -> void:
	var selected: Array = $Squads.selected
	if not selected.is_empty():
		simulation(selected[0]).order(selected[1], new_order)


func _refresh_hud() -> void:
	if _hud == null:
		return
	var seconds := simulation(lane_keys()[0]).tick_number() * _clock.tick_seconds
	var paused := _clock.is_paused()
	var chosen: Array = $Squads.selected
	var squad = null if chosen.is_empty() else simulation(chosen[0]).squad(chosen[1])
	_hud.set_status(
		FormationSkirmishReadout.status(seconds, _clock.speed_multiplier, paused),
		FormationSkirmishReadout.pool(_battle.pool, _battle.player),
		FormationSkirmishReadout.selected(chosen, squad),
		paused
	)
	var builds := _battle.player.builds()
	_hud.set_builders(
		FormationSkirmishReadout.builders(_battle.player, _builder_types, BUILDER_NAMES)
	)
	for key in lane_keys():
		var production = _battle.lane(key).production
		var line := FormationSkirmishReadout.lane(production, _battle.allowance(key), builds, key)
		_hud.set_lane(key, line[0], line[1], LANE_WIDTHS[key])


func _log(lane_key: String, event: Dictionary, text: String) -> void:
	if _hud != null:
		_hud.add_log(
			"t%d %s %s: %s" % [event.get("tick", 0), lane_key, event.get("faction", ""), text]
		)


func _banner(text: String) -> void:
	if _hud != null:
		_hud.show_banner(text)
