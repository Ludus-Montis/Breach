class_name FormationSkirmishHud
extends CanvasLayer
## Controls and readouts for the formation feel test (specs/22-formation-feel-test.md):
## time, the shared slot pool and domain reserve, the domain's builders and how their units
## are shared out (Decision 45), one brush for every lane (hotkeys 1 / 2 / 3 / E), each lane's
## wave painter with its saved presets, send and auto departure (Decision 43), the kingdom,
## orders for the selected squad, and a log. Builds its widgets in code, only emits signals.

signal pause_toggled
signal speed_chosen(multiplier: float)
signal pause_on_full_toggled(enabled: bool)
signal kingdom_auto_toggled(enabled: bool)
signal order_chosen(order: int)
signal brush_chosen(brush: int)
signal builders_changed(type_index: int, delta: int)
signal distribution_chosen(index: int)
signal preset_saved(lane_key: String)
signal preset_applied(lane_key: String, index: int)
signal cell_painted(lane_key: String, cell: Vector2i)
signal cell_erased(lane_key: String, cell: Vector2i)
signal send_wave_pressed(lane_key: String)
signal auto_departure_toggled(lane_key: String, enabled: bool)
signal auto_merge_toggled(lane_key: String, enabled: bool)
signal spawn_kingdom_pressed(lane_key: String)

## SkirmishUnit.Order's codes, as the crate's BattleState takes them.
const ADVANCE := 0
const HOLD := 1
const RETREAT := 2
const WavePainter = preload("res://presentation/skirmish/formation/wave_painter.gd")

const PANEL_WIDTH := 330.0
const LOG_LINES := 9
const BANNER_SECONDS := 3.0
## Brush choices, in order, with their hotkeys (the scene maps them to unit types; the last
## erases).
const BRUSHES := ["Grem [1]", "Brute [2]", "Spitter [3]", "Erase [E]"]

var _status: Label
var _pool: Label
var _pause_button: Button
var _selected: Label
var _log: RichTextLabel
var _banner: Label
var _brushes := []  # Button per brush, in BRUSHES order
var _builders := []  # Label per builder type
var _lanes := {}  # lane key -> {"label", "painter", "presets": OptionButton}
var _lines: PackedStringArray = []
var _banner_left := 0.0


func build(lane_keys: Array, builder_names: Array) -> void:
	var scroll := ScrollContainer.new()  # the panel is taller than small windows
	scroll.position = Vector2(10, 10)
	scroll.custom_minimum_size = Vector2(PANEL_WIDTH + 24.0, 0)
	scroll.size = Vector2(
		PANEL_WIDTH + 24.0, maxf(get_viewport().get_visible_rect().size.y - 20.0, 200.0)
	)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(scroll)
	var panel := PanelContainer.new()
	scroll.add_child(panel)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	panel.add_child(box)
	_status = _label(box, "")
	var time_row := _row(box)
	_pause_button = _button(time_row, "Pause (Space)", func(): pause_toggled.emit())
	for multiplier in [1.0, 2.0, 4.0]:
		_button(time_row, "×%d" % multiplier, func(): speed_chosen.emit(multiplier))
	_toggle(
		box,
		"Pause when a wave is full (Send resumes)",
		true,
		func(on): pause_on_full_toggled.emit(on)
	)
	_pool = _label(box, "")
	_build_domain(box, lane_keys, builder_names)
	_build_brushes(box)
	for lane_key in lane_keys:
		_build_lane(box, lane_key)
	_build_kingdom_and_orders(box, lane_keys)
	_banner = Label.new()
	_banner.position = Vector2(PANEL_WIDTH + 40.0, 14)
	_banner.add_theme_font_size_override("font_size", 20)
	add_child(_banner)


func _process(delta: float) -> void:
	if _banner_left > 0.0:
		_banner_left -= delta
		if _banner_left <= 0.0:
			_banner.text = ""


func set_status(text: String, pool_text: String, selected_text: String, paused: bool) -> void:
	_status.text = text
	_pool.text = pool_text
	_selected.text = selected_text
	_pause_button.text = "Resume (Space)" if paused else "Pause (Space)"


## One line per builder type, in the order build() was given.
func set_builders(lines: Array) -> void:
	for index in range(mini(lines.size(), _builders.size())):
		_builders[index].text = lines[index]


## Shows the chosen brush and lists the player's saved presets in every lane.
func set_tools(brush: int, preset_names: Array) -> void:
	for index in range(_brushes.size()):
		_brushes[index].set_pressed_no_signal(index == brush)
	for lane_key in _lanes:
		var picker: OptionButton = _lanes[lane_key]["presets"]
		var keep := picker.selected
		picker.clear()
		for preset_name in preset_names:
			picker.add_item(preset_name)
		if picker.item_count > 0:
			picker.select(clampi(keep, 0, picker.item_count - 1))


## text: the lane's summary; places: FormationSkirmishReadout.lane()'s places.
func set_lane(lane_key: String, text: String, places: Array, lane_width: int) -> void:
	var lane: Dictionary = _lanes[lane_key]
	lane["label"].text = text
	lane["painter"].places = places
	lane["painter"].lane_width = lane_width
	lane["painter"].queue_redraw()


func add_log(line: String) -> void:
	_lines.append(line)
	if _lines.size() > LOG_LINES:
		_lines = _lines.slice(_lines.size() - LOG_LINES)
	_log.text = "\n".join(_lines)


func show_banner(text: String) -> void:
	_banner.text = text
	_banner_left = BANNER_SECONDS


func _build_domain(box: VBoxContainer, lane_keys: Array, builder_names: Array) -> void:
	for index in range(builder_names.size()):
		var row := _row(box)
		_button(row, "−", func(): builders_changed.emit(index, -1))
		_button(row, "+", func(): builders_changed.emit(index, 1))
		var label := Label.new()
		label.text = builder_names[index]
		row.add_child(label)
		_builders.append(label)
	var share_row := _row(box)
	var share_label := Label.new()
	share_label.text = "Share units:"
	share_row.add_child(share_label)
	var picker := OptionButton.new()
	picker.focus_mode = Control.FOCUS_NONE
	for lane_key in lane_keys:
		picker.add_item("%s first" % lane_key)
	picker.add_item("Round robin")
	picker.select(lane_keys.size())  # round robin by default (Decision 51)
	picker.item_selected.connect(func(index): distribution_chosen.emit(index))
	share_row.add_child(picker)


func _build_brushes(box: VBoxContainer) -> void:
	var row := _row(box)
	var group := ButtonGroup.new()
	for index in range(BRUSHES.size()):
		var brush := _button(row, BRUSHES[index], func(): brush_chosen.emit(index))
		brush.toggle_mode = true
		brush.button_group = group
		_brushes.append(brush)
	_brushes[0].set_pressed_no_signal(true)


func _build_lane(box: VBoxContainer, lane_key: String) -> void:
	var title := "Lane P → %s  (Send: %s)" % [lane_key, "S" if lane_key == "c" else "Shift+S"]
	_label(box, title)
	var row := _row(box)
	var painter := WavePainter.new()
	painter.painted.connect(func(cell): cell_painted.emit(lane_key, cell))
	painter.erased.connect(func(cell): cell_erased.emit(lane_key, cell))
	row.add_child(painter)
	var side := VBoxContainer.new()
	side.custom_minimum_size = Vector2(PANEL_WIDTH - 150.0, 0)
	row.add_child(side)
	var label := _label(side, "")
	label.custom_minimum_size = Vector2(PANEL_WIDTH - 150.0, 0)
	var presets := OptionButton.new()
	presets.focus_mode = Control.FOCUS_NONE
	side.add_child(presets)
	var preset_row := _row(side)
	_button(preset_row, "Apply", func(): _apply_chosen(lane_key))
	_button(preset_row, "Save shape", func(): preset_saved.emit(lane_key))
	_button(side, "Send wave", func(): send_wave_pressed.emit(lane_key))
	_toggle(side, "Auto depart", true, func(on): auto_departure_toggled.emit(lane_key, on))
	_toggle(side, "Auto merge", false, func(on): auto_merge_toggled.emit(lane_key, on))
	_lanes[lane_key] = {"label": label, "painter": painter, "presets": presets}


func _apply_chosen(lane_key: String) -> void:
	var picker: OptionButton = _lanes[lane_key]["presets"]
	if picker.selected >= 0:
		preset_applied.emit(lane_key, picker.selected)


func _build_kingdom_and_orders(box: VBoxContainer, lane_keys: Array) -> void:
	var kingdom_row := _row(box)
	for lane_key in lane_keys:
		_button(
			kingdom_row, "Militia → %s" % lane_key, func(): spawn_kingdom_pressed.emit(lane_key)
		)
	_toggle(kingdom_row, "Auto", false, func(on): kingdom_auto_toggled.emit(on))
	_selected = _label(box, "Click a squad (or Tab) to select it")
	var order_row := _row(box)
	var orders := [["Advance (A)", ADVANCE], ["Hold (H)", HOLD], ["Retreat (R)", RETREAT]]
	for pair in orders:
		_button(order_row, pair[0], func(): order_chosen.emit(pair[1]))
	_log = RichTextLabel.new()
	_log.fit_content = true
	_log.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	box.add_child(_log)


func _label(parent: Container, text: String) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	parent.add_child(label)
	return label


func _row(parent: Container) -> HBoxContainer:
	var row := HBoxContainer.new()
	parent.add_child(row)
	return row


func _button(parent: Container, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(action)
	parent.add_child(button)
	return button


func _toggle(parent: Container, text: String, on: bool, action: Callable) -> void:
	var check := CheckBox.new()
	check.text = text
	check.button_pressed = on
	check.focus_mode = Control.FOCUS_NONE
	check.toggled.connect(action)
	parent.add_child(check)
