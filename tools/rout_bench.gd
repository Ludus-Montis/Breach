extends SceneTree
## How long a tick of routs takes with many friendly formations (spec 30 round 3): the
## player's side split into `squads` formations along a long lane, the front half routing
## back through the rear half standing (crushing, steering for refuge, caught, rallying),
## a kingdom formation beyond them; FormationRout.step alone timed, ms a tick.
##
##   godot --headless --path . --script res://tools/rout_bench.gd -- \
##       [side=1024] [squads=16] [width=16] [ticks=20]

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")


func _init() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var parts: PackedStringArray = arg.split("=", true, 1)
		args[parts[0]] = parts[1] if parts.size() > 1 else "true"
	var side := int(args.get("side", 1024))
	var count := int(args.get("squads", 16))
	var width := int(args.get("width", 16))
	var ticks := int(args.get("ticks", 20))
	var sim := _field(side, count, width)
	var squads := sim.squads()
	var events := 0
	var began := Time.get_ticks_usec()
	for tick in range(1, ticks + 1):
		var pace := FormationSimulation.TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE
		events += FormationRout.step(squads, tick, pace, sim.tick_seconds, null, 1).size()
	var spent := (Time.get_ticks_usec() - began) / 1000.0 / ticks
	var routing := squads.filter(func(s): return s.state == SkirmishSquad.State.ROUTING).size()
	var line := "rout: %d a side in %d formations, %d wide: %.2f ms a tick (%d events, %d routing)"
	print(line % [side, count, width, spent, events, routing])
	quit(0)


## The player's `side` units in `count` formations a few cells apart along the lane, the
## front half at no morale (they break on the first tick); a kingdom formation beyond.
func _field(side: int, count: int, width: int) -> FormationSimulation:
	var per := maxi(1, side / count)
	var depth := ceili(float(per) / width)
	var gap := depth + 4.0
	var length := ceilf((count * gap + 80.0) / MapLayoutDef.CELLS_PER_TILE)
	var sim := FormationSimulation.new(length, 0.1)
	var grem := CrateContent.unit("grem")
	var militia := CrateContent.unit("kingdom_militia")
	for index in range(count):
		var squad := sim.spawn_squad(width, _block(grem, per, width), "player", true)
		squad.front_distance = (20.0 + (index + 1) * gap) / MapLayoutDef.CELLS_PER_TILE
		squad.state = SkirmishSquad.State.HOLDING
		if index >= count / 2:
			squad.morale = 0
	var theirs := sim.spawn_squad(width, _block(militia, width * 2, width), "the_kingdom", false)
	theirs.front_distance = (20.0 + (count + 3) * gap) / MapLayoutDef.CELLS_PER_TILE
	FormationMarch.sync_units(sim.squads())  # placed there
	return sim


func _block(unit_def: Resource, count: int, width: int) -> Array:
	var placements := []
	for index in range(count):
		placements.append([unit_def, Vector2i(index / width, index % width)])
	return placements
