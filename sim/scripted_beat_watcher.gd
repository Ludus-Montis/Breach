class_name ScriptedBeatWatcher
extends RefCounted
## This map's win/loss triggers (Decision 16): reaching (capturing) the Core is an
## unconditional win. Losing the entire force loses. See
## specs/06-map-content-p-f-F-c.md.
##
## Not a generic rule-matching engine, despite the spec's original "configured rules"
## framing - with exactly one map and two rules, a small explicit API is simpler
## and just as data-independent-per-map at this scale. Genericity is worth building
## once a second map's ruleset exists to prove the abstraction against, not before.
##
## Does not self-subscribe to SimEvents (same reasoning as every other sim/
## component this slice): a composition root (Phase 3 item 11) calls these methods
## explicitly when it determines a raw SimEvents payload pertains to this map's
## specific Core.
##
## hero_party_defeated()/on_hero_party_defeated() were removed (Decision 16, found
## via manual playtest): the win condition no longer depends on the Hero Party at
## all - it remains a real, defeatable obstacle a wave can collide with en route
## (ordinary lane combat, unchanged), but its defeat is no longer a
## prerequisite for winning via the Core specifically.

var _core_node_index: int
var _defeated: bool = false
var _won: bool = false


func _init(core_node_index: int) -> void:
	_core_node_index = core_node_index


func on_node_captured(node_index: int) -> void:
	if node_index == _core_node_index and not _won:
		_won = true
		SimEvents.victory.emit()


func on_player_unit_count_changed(total_units: int) -> void:
	if total_units <= 0 and not _defeated:
		_defeated = true
		SimEvents.defeat.emit()
