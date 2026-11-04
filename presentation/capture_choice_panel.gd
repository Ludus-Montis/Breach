class_name CaptureChoicePanel
extends Control
## Ravage/Fortify/Dismantle input, per specs/08-composition-root-and-input.md. Each
## method calls exactly one issue_choice() on the crate's economy (`EconomyState`) -
## bypasses CommandQueue deliberately, same exception class as TimeControls
## (specs/05): a capture choice is a one-shot decision with nothing to slot-fill.

var capture_resolution: Object


func on_ravage_pressed(node_index: int) -> void:
	capture_resolution.issue_choice(node_index, "ravage")


func on_fortify_pressed(node_index: int) -> void:
	capture_resolution.issue_choice(node_index, "fortify")


func on_dismantle_pressed(node_index: int) -> void:
	capture_resolution.issue_choice(node_index, "dismantle")
