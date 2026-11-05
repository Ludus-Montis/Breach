class_name FormationFieldHud
extends CanvasLayer
## The 2D feel test's controls and status (spec 27): two rows of buttons - send each route's
## wave or let it go when full, or retreat it; send A down the slanted route C, send A and
## B together, have B wait for A, give the line a captain, forbid it to pursue, reset with a
## seed, pause - and a status row under it: the tick (and seconds of battle), waves built
## (leaders counted apart), the line (how far it has pursued, of its leash), the reserve
## and the battle seed. Reset starts a fresh field, with the seed typed in or a random
## one, keeping the ticked options. Below, this run's action log (the field's record)
## to copy, and a box to paste a log into: Replay restarts on its seed and plays its
## actions at their ticks, live (pause still works), counting them off. Engine glue: it
## reads the crate's field (`FieldState`, spec 39 step 9d-i).

## The field's tick, in seconds (the scene's SkirmishClock's).
const TICK_SECONDS := 0.1

const WAYS := ["north", "east", "south", "west"]
const TENDING := ["leave", "recover", "carry"]

var _scene: Node
var _autos := {}  # route key -> CheckBox
var _hurries := {}  # route key -> CheckBox (Decision 125: its waves run)
var _tends := {}  # route key -> OptionButton (Decision 126: what its waves do with the downed)
var _wait: CheckBox
var _via_c: CheckBox
var _holds: CheckBox  # the line won't pursue (it does by default, Decision 109)
var _captain: CheckBox
var _seed: LineEdit
var _status: Label
var _log_view: TextEdit
var _replay_view: TextEdit
var _replay_status: Label
## The replayed record's build against this one (SimBuild.compare) on a line of its own;
## "" if the same.
var _build_note := ""
var _replaying := 0  # how many actions the replay running holds (0: not a replay)


## Builds the controls for `scene` (FormationFieldScene).
func build(scene: Node) -> void:
	_scene = scene
	var rows := VBoxContainer.new()
	rows.position = Vector2(12, 8)
	add_child(rows)
	var bar := HBoxContainer.new()
	rows.add_child(bar)
	for key in ["A", "B"]:
		_wave_controls(bar, scene, key)
	_button(bar, "Send A+B", func(): scene.act("send A+B"))
	for key in ["A", "B"]:
		_tends[key] = _tend_choice(bar, scene, key)
	var options := HBoxContainer.new()  # a second row, so the controls fit the window
	rows.add_child(options)
	_via_c = _check(options, "A goes via C", func(on): scene.act(_toggle("via_c", on)))
	_wait = _check(options, "B waits for A", func(on): scene.act(_toggle("wait", on)))
	_captain = _check(options, "Line has a captain", func(_on): reset())
	_holds = _check(options, "Line won't pursue", func(on): scene.act(_toggle("pursues", not on)))
	_seed = LineEdit.new()
	_seed.placeholder_text = "seed (random)"
	_seed.custom_minimum_size = Vector2(110, 0)
	options.add_child(_seed)
	_button(options, "Reset", reset)
	_button(options, "Pause (Space)", scene.toggle_pause)
	_status = Label.new()
	rows.add_child(_status)
	var logs := HBoxContainer.new()
	rows.add_child(logs)
	_log_view = _log_box(logs, "This run's actions")
	_log_view.editable = false
	_button(logs, "Copy", func(): DisplayServer.clipboard_set(scene.action_log()))
	_replay_view = _log_box(logs, "Paste a log to replay")
	_button(logs, "Replay", replay)
	_replay_status = Label.new()
	logs.add_child(_replay_status)


## A route's controls: send it, send automatically, order it back, hurry it.
func _wave_controls(bar: Container, scene: Node, key: String) -> void:
	_button(bar, "Send %s" % key, func(): scene.act("send " + key))
	_autos[key] = _check(bar, "Auto %s" % key, func(on): scene.act(_toggle("auto " + key, on)))
	_button(bar, "Retreat %s" % key, func(): scene.act("retreat " + key))
	_hurries[key] = _check(bar, "Hurry %s" % key, func(on): scene.act(_toggle("hurry " + key, on)))


## A route's choice of what its waves do with their own downed: leave, recover, carry.
func _tend_choice(bar: Container, scene: Node, key: String) -> OptionButton:
	var choice := OptionButton.new()
	for mode in TENDING:
		choice.add_item("%s wounded: %s" % [key, mode])
	choice.item_selected.connect(func(index): scene.act("tend %s %s" % [key, TENDING[index]]))
	bar.add_child(choice)
	return choice


## Restarts on the pasted log's seed and replays its actions; the options are the log's.
func replay() -> void:
	var read: Dictionary = NativeLibrary.made("FieldState").read_record(_replay_view.text)
	if read.is_empty():
		_replay_status.text = 'Not a record: it starts\n"record <n> seed <n> captain <on|off>"'
		_replaying = 0
		return
	_seed.text = str(read["seed"])
	_captain.set_pressed_no_signal(read["captained"])
	for box in [_wait, _via_c, _holds] + _autos.values() + _hurries.values():
		box.set_pressed_no_signal(false)
	for choice in _tends.values():
		choice.select(0)
	_scene.restart(read["captained"], read["seed"], read["commands"])
	_replaying = read["commands"].size()
	var note := SimBuild.compare(read["build"])
	_build_note = "" if note == "" else "\n" + note


## A fresh field: the seed typed in, or a random one; the ticked options kept (and logged).
func reset() -> void:
	var text := _seed.text.strip_edges()
	var battle_seed := int(text) if text.is_valid_int() else randi() % 1000000
	_scene.restart(_captain.button_pressed, battle_seed)
	_replaying = 0
	_build_note = ""
	_replay_status.text = ""
	for key in _autos:
		if _autos[key].button_pressed:
			_scene.act(_toggle("auto " + key, true))
		if _hurries[key].button_pressed:
			_scene.act(_toggle("hurry " + key, true))
		if _tends[key].selected > 0:
			_scene.act("tend %s %s" % [key, TENDING[_tends[key].selected]])
	for option in [[_wait, "wait"], [_via_c, "via_c"]]:
		if option[0].button_pressed:
			_scene.act(_toggle(option[1], true))
	if _holds.button_pressed:
		_scene.act(_toggle("pursues", false))


## An on/off action's words.
static func _toggle(action: String, on: bool) -> String:
	return "%s %s" % [action, "on" if on else "off"]


## `field`: the scene's FieldState.
func show_status(field: Object, paused: bool, battle_seed: int) -> void:
	var built := []
	var waves: Dictionary = field.waves()
	for key in waves:
		built.append(_built(key, waves[key]))
	var squads := {}
	for squad in field.squads():
		squads[squad["id"]] = squad
	var kingdom: PackedInt64Array = field.kingdom()
	var line: Dictionary = squads[kingdom[0]]
	var halted := squads.values().any(func(s): return s["blocked"])
	_status.text = (
		"Tick %d (%.1fs)   Waves: %s   Line: %d (%s)   Reserve: %d   Seed: %d%s%s"
		% [
			field.tick_number(),
			field.tick_number() * TICK_SECONDS,
			", ".join(built),
			_living(line),
			_line_state(line),
			_living(squads[kingdom[1]]),
			battle_seed,
			"   BLOCKED" if halted else "",
			"   (paused)" if paused else "",
		]
	)
	var log: String = _scene.action_log()
	if _log_view.text != log:
		_log_view.text = log
		_log_view.scroll_vertical = _log_view.get_line_count()
	if _replaying > 0:  # the replayed actions are logged again as they're played
		var played := mini(_log_view.get_line_count() - 1, _replaying)
		var shown := [battle_seed, played, _replaying, _build_note]
		_replay_status.text = "Replaying seed %d\n%d of %d actions played%s" % shown


## "B 8/8 + leader 1/1": rank and file and leaders counted apart; `counts` [rank and file
## built, places, leaders built, places] (FieldState.waves).
static func _built(key: String, counts: PackedInt64Array) -> String:
	var text := "%s %d/%d" % [key, counts[0], counts[1]]
	if counts[3] > 0:
		text += " + leader %d/%d" % [counts[2], counts[3]]
	return text


## The line's state, the way it faces, and how far it has pursued of its leash (Decision 107).
static func _line_state(line: Dictionary) -> String:
	var state: String = line["state"].to_lower()
	if line["heading"] != null:
		state += ", faced " + WAYS[posmod(roundi(line["heading"] / 90.0), 4)]
	if line["pursuit"] != null:
		var leash: float = line["pursuit"]["leash"]
		var shown := "unleashed" if is_inf(leash) else "%d" % leash
		state += ", pursuing %d/%s cells" % [line["pursuit"]["gone"], shown]
	return state


static func _living(squad: Dictionary) -> int:
	return squad["units"].filter(func(u): return u["alive"]).size()


func _log_box(bar: Container, hint: String) -> TextEdit:
	var box := TextEdit.new()
	box.placeholder_text = hint
	box.custom_minimum_size = Vector2(260, 72)
	bar.add_child(box)
	return box


func _button(bar: Container, text: String, pressed: Callable) -> void:
	var button := Button.new()
	button.text = text
	button.pressed.connect(pressed)
	bar.add_child(button)


func _check(bar: Container, text: String, toggled: Callable) -> CheckBox:
	var box := CheckBox.new()
	box.text = text
	box.toggled.connect(toggled)
	bar.add_child(box)
	return box
