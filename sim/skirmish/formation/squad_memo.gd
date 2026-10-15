class_name SquadMemo
extends RefCounted
## One decision phase's squad geometry, worked out once a squad (and axis) rather than once
## a pair: its living units, its lateral extent (SquadGeometry.lateral), its footprints'
## reach and bounds (SquadEdges), and a grid of where its units stand (FormationSight).
## Each value is the pure function's own result, so a phase using the memo decides exactly
## as without it.
## A memo lives for one phase, made at its start and dropped at its end, while no squad
## moves; as a guard, a squad whose frame has moved (position, heading, width, centre
## shift or unit count) since its values were taken has them taken afresh. A phase must
## not change units' places in their frame or which are living, nor (for sighted) where
## they stand, while it holds a memo.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadGeometry = preload("res://sim/skirmish/formation/squad_geometry.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const EPSILON := 0.000001
## Cells a sight grid's bucket spans.
const SIGHT_CELL := 8.0

var _held := {}  # squad instance id -> {"frame": its frame, kind or [kind, axis]: value}


## squad.living().
func living(squad: SkirmishSquad) -> Array[SkirmishUnit]:
	var held := _of(squad)
	if not held.has("living"):
		held["living"] = squad.living()
	return held["living"]


## SquadEdges.reach(squad, way).
func reach(squad: SkirmishSquad, way: Vector2) -> Vector2:
	var held := _of(squad)
	var key := ["reach", way]
	if not held.has(key):
		held[key] = SquadEdges.reach(squad, way)
	return held[key]


## SquadEdges.bounds(squad).
func bounds(squad: SkirmishSquad) -> Rect2:
	var held := _of(squad)
	if not held.has("bounds"):
		held["bounds"] = SquadEdges.bounds(squad)
	return held["bounds"]


## SquadGeometry.overlaps(from, to).
func overlaps(from: SkirmishSquad, to: SkirmishSquad) -> bool:
	var apart := fposmod(from.heading - to.heading, 180.0)
	var parallel := SquadGeometry.PARALLEL
	if apart > parallel + EPSILON and apart < 180.0 - parallel - EPSILON:
		return false
	var axis := SquadFrame.lateral_axis(from.heading)
	var mine := _lateral(from, axis)
	var theirs := _lateral(to, axis)
	return minf(mine.y, theirs.y) - maxf(mine.x, theirs.x) > EPSILON


## SquadEdges.face_gap(from, to).
func face_gap(from: SkirmishSquad, to: SkirmishSquad) -> float:
	var ahead := UnitMotion.vector(from.heading)
	var near := reach(to, ahead).x
	return (near - from.position.dot(ahead)) / MapLayoutDef.CELLS_PER_TILE


## SquadEdges.overlap_across(from, to).
func overlap_across(from: SkirmishSquad, to: SkirmishSquad) -> bool:
	var axis := SquadFrame.lateral_axis(from.heading)
	var mine := _lateral(from, axis)
	var theirs := reach(to, axis)
	return minf(mine.y, theirs.y) - maxf(mine.x, theirs.x) > EPSILON


## {"cells": {Vector2i: [unit position, ...]}, "low", "high": the occupied cells' corners}:
## where the squad's living units stand, in SIGHT_CELL buckets.
func sighted(squad: SkirmishSquad) -> Dictionary:
	var held := _of(squad)
	if not held.has("sighted"):
		var cells := {}
		var low := Vector2i(1 << 30, 1 << 30)
		var high := -low
		var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
		for unit in living(squad):
			var cell := Vector2i((at[unit.slot] / SIGHT_CELL).floor())
			if not cells.has(cell):
				cells[cell] = []
				low = low.min(cell)
				high = high.max(cell)
			cells[cell].append(at[unit.slot])
		held["sighted"] = {"cells": cells, "low": low, "high": high}
	return held["sighted"]


## SquadGeometry.lateral(squad, axis).
func _lateral(squad: SkirmishSquad, axis: Vector2) -> Vector2:
	var held := _of(squad)
	var key := ["lateral", axis]
	if not held.has(key):
		held[key] = SquadGeometry.lateral(squad, axis)
	return held[key]


## The squad's held values, emptied first if its frame has moved since they were taken.
func _of(squad: SkirmishSquad) -> Dictionary:
	var frame := [
		squad.position, squad.heading, squad.width, squad.centre_shift, squad.units.size()
	]
	var held: Dictionary = _held.get(squad.get_instance_id(), {})
	if held.get("frame") != frame:
		held = {"frame": frame}
		_held[squad.get_instance_id()] = held
	return held
