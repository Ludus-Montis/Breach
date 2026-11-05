class_name FormationBench
extends RefCounted
## How long the formation fight takes a tick as battles grow (spec 30 round 3): grems
## against kingdom militia, head-on along the lane, the same number a side, in blocks of
## the given width. Times a tick marching (before any contact) and mid-fight (once the
## fight has settled in), so a speed-up can be judged and a slow-down caught
## (the crate's benches since spec 39 step 9d-iii). Reads the clock, so not part of any battle.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")

## Ticks a battle may take to come to contact before it is given up on.
const CONTACT_LIMIT := 2000
## Cells between neighbouring lanes' centre lines (lanes()).
const LANE_SPACING := 12.0
## The mixed ground (spec 38 step 2's `terrain=mixed`), 2 tiles by 1, its line of advance
## along MIXED_Y: a wood 16 cells deep across the whole ground, a quarter of the way from
## the player's end, and a stream too deep for grems three quarters of the way, with a ford
## 32 cells wide where the line crosses it. Fixed, so runs compare.
const MIXED_SIZE := Vector2i(128, 64)
const MIXED_Y := 32.0
const MIXED_WOOD := Rect2i(32, 0, 16, 64)
const MIXED_STREAM := Rect2i(80, 0, 3, 64)
const MIXED_FORD := Rect2i(80, 16, 3, 32)
const MIXED_STREAM_DEPTH := 1.2
const MIXED_FORD_DEPTH := 0.4


## {"march": [avg ms, worst ms], "fight": [avg ms, worst ms], "contact": tick (-1 if none)}
## for `per_side` a side; options: width, march, settle, fight (ticks), seed.
static func measure(per_side: int, options: Dictionary) -> Dictionary:
	var sim := clash(per_side, options.get("width", 32), options.get("seed", 1))
	var marching := []
	var contact := -1
	while contact < 0 and sim.tick_number() < CONTACT_LIMIT:
		var began := Time.get_ticks_usec()
		var events := sim.step()
		if events.any(func(e): return e["type"] == "engaged"):
			contact = sim.tick_number()
		elif marching.size() < options.get("march", 10):
			marching.append((Time.get_ticks_usec() - began) / 1000.0)
	var fighting := []
	if contact >= 0:
		for _i in range(options.get("settle", 10)):
			sim.step()
		for _i in range(options.get("fight", 10)):
			var began := Time.get_ticks_usec()
			sim.step()
			fighting.append((Time.get_ticks_usec() - began) / 1000.0)
	return {"march": _spread(marching), "fight": _spread(fighting), "contact": contact}


## The battle: `per_side` grems at the player's end against as many militia, `width` wide;
## the grems' last `archers` (their back ranks) spitters, which shoot.
static func clash(
	per_side: int, width: int, battle_seed: int, archers: int = 0
) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = battle_seed
	_spawn_pair(sim, [per_side, width, archers], null)
	return sim


## Many squads: `lanes` parallel lanes LANE_SPACING cells apart, each with a squad of
## `per_squad` grems against as many militia, `width` wide (the grems' last `archers`
## spitters), so the per-pair passes meet many squads at once.
static func lanes(
	lane_count: int, per_squad: int, width: int, battle_seed: int, archers: int = 0
) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = battle_seed
	var length := 2.0 * MapLayoutDef.CELLS_PER_TILE
	for lane in range(lane_count):
		var y := lane * LANE_SPACING
		var route := FormationRoute.new(PackedVector2Array([Vector2(0, y), Vector2(length, y)]))
		_spawn_pair(sim, [per_squad, width, archers], route)
	return sim


## FormationBench.clash on the mixed ground (mixed_ground()), its line of advance along
## MIXED_Y, through the middle of the ground.
static func mixed(
	per_side: int, width: int, battle_seed: int, archers: int = 0
) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = battle_seed
	sim.terrain = mixed_ground()
	var length := 2.0 * MapLayoutDef.CELLS_PER_TILE
	var line := PackedVector2Array([Vector2(0, MIXED_Y), Vector2(length, MIXED_Y)])
	_spawn_pair(sim, [per_side, width, archers], FormationRoute.new(line))
	return sim


## The mixed ground: the wood as FormationField's (half pace, blocks sight, the tuning's
## wood obscurance), then the stream, then the ford painted over it.
static func mixed_ground() -> FormationTerrain:
	var terrain := FormationTerrain.new(MIXED_SIZE)
	var obscurance := BattleTuning.current().sight_wood_obscurance
	terrain.paint(MIXED_WOOD, {"cost": 0.5, "blocks_sight": true, "obscurance": obscurance})
	terrain.paint(MIXED_STREAM, {"depth": MIXED_STREAM_DEPTH})
	terrain.paint(MIXED_FORD, {"depth": MIXED_FORD_DEPTH})
	return terrain


## A grem squad at the player's end and a militia one at the other; `shape` = [units,
## width, archers].
static func _spawn_pair(sim: FormationSimulation, shape: Array, route: FormationRoute) -> void:
	var grem: Resource = CrateContent.unit("grem")
	var spitter: Resource = CrateContent.unit("grem_spitter")
	var sides := [
		[grem, "player", true],
		[CrateContent.unit("kingdom_militia"), "the_kingdom", false],
	]
	var count: int = shape[0]
	var width: int = shape[1]
	for side in sides:
		var placements := []
		for index in range(count):
			var shoots: bool = side[0] == grem and index >= count - shape[2]
			placements.append(
				[spitter if shoots else side[0], Vector2i(index / width, index % width)]
			)
		sim.spawn_squad(width, placements, side[1], side[2], 0, route)


## [average, worst] of the times, or [0, 0] for none.
static func _spread(times: Array) -> Array:
	if times.is_empty():
		return [0.0, 0.0]
	return [times.reduce(func(a, t): return a + t, 0.0) / times.size(), times.max()]
