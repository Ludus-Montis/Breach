class_name TerrainObscurance
extends RefCounted
## How much each cell of the ground hides what lies beyond it (spec 30): a view over the
## crate's ground (`TerrainGround`, native/rust/src/ground/sight.rs, spec 39 step 3a), one
## rule for sight. Each cell has an obscurance - what a cell of distance through it costs a
## sight line - and `fog`, the field's weather, adds its own to every cell. A sight line
## sums the obscurance cell by cell, each cell's by the distance the line runs through it,
## and sight ends where the sum passes the looker's sight budget (BattleTuning.sight_budget).
## FormationTerrain paints the cells; PathSight plans by it.

## The weather's obscurance, added to every cell's.
var fog: float:
	get:
		return _ground.fog()
	set(value):
		_ground.set_fog(value)

var _ground: Object


func _init(ground: Object) -> void:
	_ground = ground


## The obscurance of the cell holding `point` (the mean of the cells either side where it
## lies on a cell edge), with the fog.
func at(point: Vector2) -> float:
	return _ground.obscurance_at(point)


## The obscurance summed along the sight line from `from` to `to`: each cell's times the
## distance the line runs through it, cell by cell. It stops counting once past `limit`.
func along(from: Vector2, to: Vector2, limit: float = INF) -> float:
	return _ground.sight_along(from, to, limit)


## Whether a looker with sight budget `budget` at `from` sees `to`.
func clear(from: Vector2, to: Vector2, budget: float) -> bool:
	return _ground.sight_clear(from, to, budget)
