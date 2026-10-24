extends GdUnitTestSuite
## The grid the passes still in GDScript search through (BodyGrid, spec 30 round 3): its
## rings hold every point once. Steering's look through its crowd and the slot search's own
## grids are the crate's (`near.rs`, `regroup/tests.rs`, `scrum.rs`; spec 39 steps 4a and 4c).

const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")


func test_a_grids_rings_hold_every_point_once() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 3
	var points := []
	for _i in range(200):
		points.append(Vector2(rng.randf_range(-30, 30), rng.randf_range(5, 25)))
	var grid := BodyGrid.build(points)
	var centre := BodyGrid.cell_of(Vector2(-50, 0))
	var span := BodyGrid.ring_span(grid, centre)
	var seen := []
	for ring_number in range(span.y + 1):
		var found := BodyGrid.ring(grid, centre, ring_number)
		assert_bool(ring_number >= span.x or found.is_empty()).is_true()
		seen.append_array(found)
	seen.sort()
	assert_array(seen).is_equal(range(points.size()))
