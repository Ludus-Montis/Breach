class_name FormationTerrain
extends RefCounted
## The ground a formation fight is on (Decision 85, spec 27 round 4): a view over the
## crate's ground (`TerrainGround`, native/rust/src/ground/, spec 39 step 3a), which holds
## the cells and their rules - move cost, height (in quarters of a cell), liquid depth (in
## cells), flow, climb difficulty, whether a cell blocks sight and its obscurance (with the
## field's fog: how far sight reaches) - and how fast a walker crosses them (`crossing`:
## ground, slope, liquid in bands of its height, climbing and swimming by Decision 64's pair
## rule). Outside the grid is open, level ground. The shares and heights are BattleTuning's
## (`ground_*`), read once when the ground is made. The field paints it; the simulation and
## TerrainPaths read it.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const TerrainObscurance = preload("res://sim/skirmish/formation/terrain_obscurance.gd")

const GROUND_CLASS := "TerrainGround"

var size: Vector2i
## Each kind of walker's cached speed shares (WalkerShares), dropped when it is painted.
var walker_shares := {}
## The fastest ground painted (a road's cost above 1): searches divide their estimates by
## it, so a quicker way along a road is never overlooked (PathSearch).
var fastest: float:
	get:
		return _ground.fastest()
## How much each cell hides what lies beyond it, and the fog (spec 30).
var obscurance: TerrainObscurance

var _ground: Object


func _init(grid_size: Vector2i) -> void:
	size = grid_size
	NativeLibrary.load_for(GROUND_CLASS)
	_ground = ClassDB.instantiate(GROUND_CLASS)
	var tuning := BattleTuning.current()
	var shares := PackedFloat64Array(
		[tuning.ground_cliff_quarters, tuning.ground_slope_cost, tuning.ground_wading]
	)
	shares.append_array(
		[tuning.ground_slow_wading, tuning.ground_swim_pace, tuning.ground_climb_pace]
	)
	shares.append(tuning.ground_climb_demand)
	_ground.setup(size, shares)
	obscurance = TerrainObscurance.new(_ground)


## Sets the cells of `area`: any of "cost", "height" (quarters), "depth" (cells), "flows"
## (its liquid's flow), "climb" (the climb difficulty of a cliff rising into them),
## "obscurance" (what a cell of sight through them costs) and "blocks_sight".
func paint(area: Rect2i, props: Dictionary) -> void:
	walker_shares.clear()
	_ground.paint(area, props)


## The height (quarters) of the cell holding a point.
func height_at(at: Vector2) -> int:
	return _ground.height_at(at)


func blocks_sight(at: Vector2) -> bool:
	return _ground.blocks_sight(at)


## How fast a unit goes stepping from `from` into `to`, as a share of its speed on open
## level ground; 0 where it can't go. `who` is a unit - crossing as its own walker
## (TerrainWalker.of_unit: swimming, climbing) - or a bare height (a grounded walker).
func factor(who, from: Vector2, to: Vector2) -> float:
	var walker: TerrainWalker = (
		TerrainWalker.grounded(who) if who is float or who is int else TerrainWalker.of_unit(who)
	)
	return crossing(walker, from, to)


## How fast `walker` goes stepping from `from` into `to`, as a share of its speed on open
## level ground; 0 where it can't go.
func crossing(walker: TerrainWalker, from: Vector2, to: Vector2) -> float:
	return _ground.crossing(
		walker.height, walker.swimmer, walker.climber, walker.swims, walker.climbs, from, to
	)


## The movement mode of `walker` stepping from `from` into `to`: climbing a cliff, else
## swimming water at least its height deep, else walking (spec 30).
func mode(walker: TerrainWalker, from: Vector2, to: Vector2) -> TerrainWalker.Mode:
	return _ground.mode(
		walker.height, walker.swimmer, walker.climber, walker.swims, walker.climbs, from, to
	)


## True if `from` stands at least a quarter-cell higher than `to`: a striker there has
## the high ground (Decision 85).
func high_ground(from: Vector2, to: Vector2) -> bool:
	return _ground.high_ground(from, to)
