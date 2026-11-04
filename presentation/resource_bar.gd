class_name ResourceBar
extends Control
## Read-only pool readout of the crate's economy (`EconomyState`), per
## specs/05-presentation-and-hud.md. No
## automated coverage beyond construction (see specs/05's Notes) - actual rendering
## needs a real render context, deferred to manual playtest once a running scene
## exists (item 11).

const RESOURCES := ["food", "wood", "stone", "metal", "crystal"]

var economy: Object


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	if economy == null:
		return
	var y := 0.0
	for resource in RESOURCES:
		draw_string(
			ThemeDB.fallback_font,
			Vector2(0.0, y),
			"%s: %d" % [resource.capitalize(), economy.balance(resource)]
		)
		y += 16.0
