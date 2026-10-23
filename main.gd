extends Node2D
## Composition root for the P-f-F-c vertical slice (Phase 3, item 11). Wires every
## sim/ and presentation/ component together against the authored map, and hosts the
## minimal input surface (specs/08-composition-root-and-input.md). Engine glue only -
## no automated coverage of this file itself, per specs/08's own Notes; verified by
## the scripted headless run recorded in specs/06 and by manual playtest.

const LaneSimulation = preload("res://sim/lane_simulation.gd")
const EconomySystem = preload("res://sim/economy_system.gd")
const CaptureResolution = preload("res://sim/capture_resolution.gd")
const SuspicionSystem = preload("res://sim/suspicion_system.gd")
const TaskForceDispatch = preload("res://sim/task_force_dispatch.gd")
const ScriptedBeatWatcher = preload("res://sim/scripted_beat_watcher.gd")
const SimulationClock = preload("res://sim/simulation_clock.gd")
const CommandQueue = preload("res://sim/command_queue.gd")
const LaneView = preload("res://presentation/lane_view.gd")
const ResourceBar = preload("res://presentation/resource_bar.gd")
const WaveCommandPanel = preload("res://presentation/wave_command_panel.gd")
const TimeControls = preload("res://presentation/time_controls.gd")
const WaveFormPanel = preload("res://presentation/wave_form_panel.gd")
const CaptureChoicePanel = preload("res://presentation/capture_choice_panel.gd")
const NarrativeLog = preload("res://presentation/narrative_log.gd")
const TickProgressIndicator = preload("res://presentation/tick_progress_indicator.gd")
const PlayerRoster = preload("res://sim/player_roster.gd")
const HERO_PARTY_DEF: ResponseUnitDef = preload("res://content/response_units/hero_party.tres")

const P := 0
const FARM := 1
const FORT := 2
const CORE := 4

const FORT_ENGAGE_SPIKE := 50.0
const FORT_CAPTURE_SPIKE := 25.0

var _map: MapDef = CrateContent.map("p_f_F_c")
var _lane: LaneSimulation
var _economy: EconomySystem
var _capture: CaptureResolution
var _suspicion: SuspicionSystem
var _dispatch: TaskForceDispatch
var _watcher: ScriptedBeatWatcher
var _clock: SimulationClock
var _queue: CommandQueue
var _roster: PlayerRoster

var _wave_form: WaveFormPanel
var _capture_choice: CaptureChoicePanel
var _wave_command_panel: WaveCommandPanel
var _narrative_label: RichTextLabel
var _time_controls_lane_view: LaneView
var _ravage_button: Button
var _fortify_button: Button
var _dismantle_button: Button
var _time_controls: TimeControls
var _auto_pause_button: Button
var _tick_indicator: TickProgressIndicator
var _pending_units_label: Label
var _grem: UnitDef = CrateContent.unit("grem")


func _ready() -> void:
	_build_simulation()
	_build_presentation()
	_connect_events()
	_lane.spawn_wave(
		"player",
		[
			{"hp": _grem.hp, "dmg": _grem.melee_damage()},
			{"hp": _grem.hp, "dmg": _grem.melee_damage()},
			{"hp": _grem.hp, "dmg": _grem.melee_damage()}
		],
		P,
		1
	)
	_update_choice_button_visibility()


func _process(delta: float) -> void:
	_clock.advance_time(delta)


func _build_simulation() -> void:
	_lane = LaneSimulation.new(_map.lanes[0].nodes)
	_economy = EconomySystem.new()
	_capture = CaptureResolution.new(_economy)
	_suspicion = SuspicionSystem.new(_map.suspicion_tier_thresholds, _map.suspicion_decay_per_tick)
	_dispatch = TaskForceDispatch.new(
		_lane,
		CORE,
		-1,
		{SuspicionSystem.Tier.MOBILIZED: {"hp": HERO_PARTY_DEF.hp, "dmg": HERO_PARTY_DEF.dmg}}
	)
	_watcher = ScriptedBeatWatcher.new(CORE)
	_clock = SimulationClock.new()
	_clock.tick_duration_seconds = _map.tick_duration_seconds
	_clock.auto_pause_each_tick = true
	_queue = CommandQueue.new()
	_roster = PlayerRoster.new()


func _build_presentation() -> void:
	_build_lane_view()
	_build_tick_indicator()
	_build_readouts()
	_build_time_controls()
	_build_wave_form_input()
	_build_capture_choice_input()
	_narrative_label = RichTextLabel.new()
	_narrative_label.position = Vector2(20, 320)
	_narrative_label.custom_minimum_size = Vector2(700, 220)
	add_child(_narrative_label)


func _build_tick_indicator() -> void:
	_tick_indicator = TickProgressIndicator.new()
	_tick_indicator.tick_duration_seconds = _map.tick_duration_seconds
	_tick_indicator.position = Vector2(20, 100)
	add_child(_tick_indicator)


func _build_lane_view() -> void:
	var lane_view := LaneView.new()
	lane_view.lane = _lane
	lane_view.tick_duration_seconds = _map.tick_duration_seconds
	lane_view.position = Vector2(400, 100)
	add_child(lane_view)
	_time_controls_lane_view = lane_view


func _build_readouts() -> void:
	var resource_bar := ResourceBar.new()
	resource_bar.economy = _economy
	resource_bar.position = Vector2(20, 20)
	add_child(resource_bar)

	_wave_command_panel = WaveCommandPanel.new()
	_wave_command_panel.command_queue = _queue
	_wave_command_panel.position = Vector2(20, 140)
	add_child(_wave_command_panel)


func _build_time_controls() -> void:
	_time_controls = TimeControls.new()
	_time_controls.simulation_clock = _clock
	_time_controls.lane_view = _time_controls_lane_view
	add_child(_time_controls)
	_add_button("Pause/Resume", Vector2(20, 180), _time_controls.on_pause_pressed)
	_add_button("Skip to next tick", Vector2(180, 180), _time_controls.on_skip_pressed)
	_add_button("1x", Vector2(340, 180), func(): _on_speed_button_pressed(1.0))
	_add_button("2x", Vector2(390, 180), func(): _on_speed_button_pressed(2.0))
	_add_button("4x", Vector2(440, 180), func(): _on_speed_button_pressed(4.0))
	_auto_pause_button = _add_button(
		"Auto-pause: ON", Vector2(500, 180), _on_auto_pause_button_pressed
	)


func _build_wave_form_input() -> void:
	_wave_form = WaveFormPanel.new()
	_wave_form.economy = _economy
	_wave_form.command_queue = _queue
	add_child(_wave_form)
	_add_button("Queue Grem (8 food)", Vector2(20, 220), _on_queue_grem_button_pressed)
	_add_button("March wave", Vector2(220, 220), _on_march_pressed)
	_pending_units_label = Label.new()
	_pending_units_label.position = Vector2(420, 225)
	_pending_units_label.text = "Pending: 0"
	add_child(_pending_units_label)


func _build_capture_choice_input() -> void:
	_capture_choice = CaptureChoicePanel.new()
	_capture_choice.capture_resolution = _capture
	add_child(_capture_choice)
	_ravage_button = _add_button(
		"Ravage Farm", Vector2(20, 260), func(): _capture_choice.on_ravage_pressed(FARM)
	)
	_fortify_button = _add_button(
		"Fortify Fort", Vector2(220, 260), func(): _capture_choice.on_fortify_pressed(FORT)
	)
	_dismantle_button = _add_button(
		"Dismantle Fort", Vector2(420, 260), func(): _capture_choice.on_dismantle_pressed(FORT)
	)


func _add_button(text: String, at: Vector2, callback: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.position = at
	button.pressed.connect(callback)
	add_child(button)
	return button


func _on_speed_button_pressed(multiplier: float) -> void:
	_time_controls.on_speed_selected(multiplier)
	_time_controls_lane_view.speed_multiplier = multiplier
	_tick_indicator.speed_multiplier = multiplier


func _on_auto_pause_button_pressed() -> void:
	var new_state := not _clock.auto_pause_each_tick
	_time_controls.on_auto_pause_toggled(new_state)
	_auto_pause_button.text = "Auto-pause: %s" % ("ON" if new_state else "OFF")


func _on_queue_grem_button_pressed() -> void:
	_wave_form.on_queue_grem_pressed(_grem)
	_pending_units_label.text = "Pending: %d" % _wave_form.pending_count()


func _on_march_pressed() -> void:
	_wave_form.on_march_pressed(P, 1)
	var pending := _queue.pending_commands()
	if pending.size() > 0:
		_wave_command_panel.command_id = pending[-1]
	_pending_units_label.text = "Pending: %d" % _wave_form.pending_count()


func _connect_events() -> void:
	SimEvents.tick_advanced.connect(_on_tick_advanced)
	SimEvents.suspicion_tier_changed.connect(_on_suspicion_tier_changed)
	SimEvents.node_captured.connect(_on_node_captured)
	SimEvents.combat_resolved.connect(_on_combat_resolved)
	SimEvents.command_committed.connect(_on_command_committed)
	SimEvents.victory.connect(_on_victory)
	SimEvents.defeat.connect(_on_defeat)


func _on_tick_advanced(tick_number: int) -> void:
	_queue.on_tick_advanced(tick_number)
	_capture.on_tick_advanced(tick_number)
	_suspicion.on_tick_advanced(tick_number)
	var pre_movers := _lane.moving_blockers().size()
	_lane.advance_positions()
	if _dispatch.dispatched().size() > 0 and _lane.moving_blockers().size() < pre_movers:
		_dispatch.mark_consumed(_dispatch.dispatched()[0])
	_watcher.on_player_unit_count_changed(_player_unit_count())
	_update_choice_button_visibility()


func _player_unit_count() -> int:
	var marching := 0
	for wave in _lane.waves():
		if wave["owner"] == "player":
			marching += wave["units"].size()
	return _roster.total(marching)


func _update_choice_button_visibility() -> void:
	_ravage_button.visible = _lane.node_state(FARM)["owner"] == "player"
	_fortify_button.visible = _capture.is_awaiting_choice(FORT)
	_dismantle_button.visible = _capture.is_awaiting_choice(FORT)


func _on_node_captured(node_index: int) -> void:
	_capture.on_node_captured(node_index, _map.lanes[0].nodes[node_index])
	if node_index == FARM:
		_despawn_player_wave_at(FARM)
	if node_index == FORT:
		_suspicion.add_suspicion(FORT_CAPTURE_SPIKE)
	_watcher.on_node_captured(node_index)


func _despawn_player_wave_at(index: int) -> void:
	for wave in _lane.waves().duplicate():
		if wave["position"] == index and wave["owner"] == "player":
			_roster.mark_harvesting(wave["units"].size())
			_lane.despawn_wave(wave)


func _on_combat_resolved(node_index: int, _outcome: Dictionary) -> void:
	if node_index == FORT:
		_suspicion.add_suspicion(FORT_ENGAGE_SPIKE)


func _on_command_committed(command: Dictionary) -> void:
	if command.get("type") != "wave":
		return
	_lane.spawn_wave(
		command["owner"], command["units"], command["start_index"], command["direction"]
	)


func _on_suspicion_tier_changed(tier: int) -> void:
	_dispatch.on_tier_entered(tier)
	var message := NarrativeLog.message_for_tier(tier)
	if message != "":
		_narrative_label.append_text(message + "\n")


func _on_victory() -> void:
	_narrative_label.append_text("VICTORY! The horde has reached the Core.\n")
	_clock.pause()


func _on_defeat() -> void:
	_narrative_label.append_text("DEFEAT. The standing force has been wiped out.\n")
	_clock.pause()
