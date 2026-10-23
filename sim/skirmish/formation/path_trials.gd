class_name PathTrials
extends RefCounted
## Paths over the feel test's field (spec 30, TerrainPaths): its grass, wood, stream with
## a 4-cell ford and hill (FormationField's), with a crag added - a 2-cell cliff across the
## south, climb difficulty 2, open to the north - for a climber to climb. Four walkers go
## each trip: a grem, a militiaman loaded past the mode load, a cart that neither swims nor
## climbs and a grem climber 2. Two trips see the whole field; two plan on what a grem
## sees (its sight shape, facing east): 40 cells ahead towards the far side, and rejoining
## route A from 13 cells off it. Each search is timed (the mean of `repeats`, warm share
## cache) and the whole-field ones drawn as text: '.' grass, 'T' wood, '=' wading water,
## '~' deep water, '^' hill, '#' crag, '*' the way.

const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const TerrainPaths = preload("res://sim/skirmish/formation/terrain_paths.gd")
const PathSearch = preload("res://sim/skirmish/formation/path_search.gd")
const PathSight = preload("res://sim/skirmish/formation/path_sight.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")

const GREM := "grem"
const CRAG := Rect2i(70, 26, 4, 31)
const CRAG_QUARTERS := 8
const CRAG_CLIMB := 2
## Trip -> {"from", and "to" (a point) or "route" (route A); "sight": planned on what a
## grem facing east sees - the wood hiding all but its first few cells - else on the whole
## field}.
const TRIPS := {
	"across": {"from": Vector2(2.5, 40.5), "to": Vector2(125.5, 40.5), "sight": false},
	"stream": {"from": Vector2(20.5, 6.5), "to": Vector2(40.5, 6.5), "sight": false},
	"sight 40 ahead": {"from": Vector2(20.5, 40.5), "to": Vector2(125.5, 40.5), "sight": true},
	"sight into wood": {"from": Vector2(8.5, 12.5), "to": Vector2(125.5, 12.5), "sight": true},
	"rejoin route A": {"from": Vector2(50.5, 45.5), "route": true, "sight": true},
}
const WALKERS := ["grem", "loaded militiaman", "cart (sinks)", "climber"]
const FIELD_VIEW := Rect2i(0, 0, 128, 64)
const STREAM_VIEW := Rect2i(14, 0, 34, 26)
## What the militiaman carries on top of his spear: past the mode load.
const PACK_WEIGHT := 14.0


## A row a trip and walker: "trip", "walker", "cells", "waypoints", "reaches", "seconds"
## (along the way at its speed), "search_usec" (the mean of `repeats` searches) and
## "expanded" (cells searched).
static func run(repeats: int) -> Array:
	var terrain := ground()
	var rows := []
	for trip in TRIPS:
		for name in WALKERS:
			var unit_def := _unit(name)
			var walker := TerrainWalker.of(unit_def)
			var row := _timed(terrain, walker, TRIPS[trip], repeats)
			row.merge({"trip": trip, "walker": name})
			row["seconds"] = TerrainPaths.cost(terrain, walker, row["waypoints"]) / unit_def.speed
			rows.append(row)
	return rows


## The field's ground with the crag.
static func ground() -> FormationTerrain:
	var grem: UnitDef = CrateContent.unit(GREM)
	var terrain: FormationTerrain = FormationField.new(0.1, grem, 0, grem).sim.terrain
	terrain.paint(CRAG, {"height": CRAG_QUARTERS, "climb": CRAG_CLIMB})
	return terrain


## `view` of `terrain` (the field's by default) drawn a character a `scale` x `scale`
## block, with `cells` on it and `cliffs` (the crag by default) marked.
static func drawing(
	cells: Array, view: Rect2i, scale: int, terrain: FormationTerrain = null, cliffs: Array = [CRAG]
) -> PackedStringArray:
	var shown := ground() if terrain == null else terrain
	var on_way := {}
	for cell in cells:
		on_way[Vector2i(cell) / scale] = true
	var out := PackedStringArray()
	for row in range(view.position.y / scale, view.end.y / scale):
		var line := ""
		for column in range(view.position.x / scale, view.end.x / scale):
			var block := Vector2i(column, row)
			var mark := _ground_mark(shown, block * scale)
			if cliffs.any(func(cliff): return cliff.has_point(block * scale)):
				mark = "#"
			line += "*" if on_way.has(block) else mark
		out.append(line)
	return out


## The trial's lines: a table of the trips, and the whole-field ways drawn.
static func report(repeats: int) -> PackedStringArray:
	var rows := run(repeats)
	var out := PackedStringArray(
		[
			(
				"%-16s %-18s %6s %5s %8s %8s %9s %9s"
				% ["trip", "walker", "cells", "legs", "reaches", "seconds", "searched", "search us"]
			)
		]
	)
	for row in rows:
		(
			out
			. append(
				(
					"%-16s %-18s %6d %5d %8s %8.1f %9d %9d"
					% [
						row["trip"],
						row["walker"],
						row["cells"].size(),
						maxi(0, row["waypoints"].size() - 1),
						"yes" if row["reaches"] else "edge",
						row["seconds"],
						row["expanded"],
						row["search_usec"],
					]
				)
			)
		)
	for row in rows:
		var view: Variant = {"across": FIELD_VIEW, "stream": STREAM_VIEW}.get(row["trip"])
		if view != null:
			out.append("\n%s, %s:" % [row["trip"], row["walker"]])
			out.append_array(drawing(row["cells"], view, 2 if view == FIELD_VIEW else 1))
	return out


static func _timed(
	terrain: FormationTerrain, walker: TerrainWalker, trip: Dictionary, repeats: int
) -> Dictionary:
	var from: Vector2 = trip["from"]
	var start := Vector2i(floori(from.x), floori(from.y))
	var grem: UnitDef = CrateContent.unit(GREM)
	var sight: PathSight = (
		PathSight.of(grem, from, Vector2.RIGHT, terrain) if trip["sight"] else null
	)
	var route := FormationRoute.new(FormationField.route_points()["A"])
	var search: PathSearch
	var began := Time.get_ticks_usec()
	for _repeat in range(maxi(1, repeats)):
		search = PathSearch.new(terrain, walker, start, sight)
		if trip.has("route"):
			search.to_route(route, route.distance_of(from))
		else:
			search.to_cell(Vector2i(floori(trip["to"].x), floori(trip["to"].y)))
	var searched := (Time.get_ticks_usec() - began) / maxi(1, repeats)
	var planned := (
		TerrainPaths.rejoin(terrain, walker, from, route, sight)
		if trip.has("route")
		else TerrainPaths.plan(terrain, walker, from, trip["to"], sight)
	)
	planned.merge({"expanded": search.expanded, "search_usec": searched})
	return planned


static func _unit(name: String) -> UnitDef:
	var unit_def: UnitDef = CrateContent.unit(GREM).duplicate()
	if name == "loaded militiaman":
		unit_def = CrateContent.unit("kingdom_militia").duplicate()
		var pack := ItemDef.new()
		pack.item_name = "pack"
		pack.weight = PACK_WEIGHT
		var items := unit_def.items.duplicate()
		items.append(pack)
		unit_def.items = items
	elif name == "cart (sinks)":
		unit_def.traits = {"sinks": 1, "cant_climb": 1}
	elif name == "climber":
		unit_def.traits = {"climber": CRAG_CLIMB}
	return unit_def


static func _ground_mark(terrain: FormationTerrain, cell: Vector2i) -> String:
	var centre := Vector2(cell) + Vector2(0.5, 0.5)
	var pace := terrain.factor(1.0, centre, centre)
	if pace <= 0.0:
		return "~"
	if terrain.blocks_sight(centre):
		return "T" if pace >= 0.5 else "="
	if pace < 1.0:
		return "="
	return "^" if terrain.height_at(centre) > 0 else "."
