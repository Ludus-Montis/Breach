class_name WaveCommandPanel
extends Control
## Live wave-command status, per specs/05-presentation-and-hud.md. Shows
## pending/committed status only, not a numeric "X/Y filled" count - see this
## project's Notes for why (CommandQueue's is_filled_check is a boolean predicate by
## design, with no numeric progress source to read from yet; nothing in this slice's
## content authors a "wave size" for that count to be measured against either).
## Revisit once real wave-forming content exists to drive it.

## The lane (`LaneState`), which holds the command queue (spec 39 step 9c).
var command_queue: Object
var command_id: int = -1


func status_text(is_filled: bool) -> String:
	return "Ready to commit" if is_filled else "Waiting for more units"


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	if command_queue == null or command_id < 0:
		return
	draw_string(
		ThemeDB.fallback_font, Vector2.ZERO, status_text(command_queue.is_filled(command_id))
	)
