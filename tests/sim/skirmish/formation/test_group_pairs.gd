extends GdUnitTestSuite
## GroupPairs (spec 38 step 1a): the pairs of groups near enough for forming up to look at,
## found through a grid, every one that trying every pair would find, in list order.

const GroupPairs = preload("res://sim/skirmish/formation/group_pairs.gd")


func _groups(points: Array) -> Array:
	return points.map(func(point): return {"position": point})


## Every pair i < j within reach, found the slow way.
func _every_pair(groups: Array, reaches: Array, join: float) -> Array:
	var pairs := []
	for i in groups.size():
		for j in range(i + 1, groups.size()):
			var apart: float = groups[i].position.distance_to(groups[j].position)
			if apart <= reaches[i] + reaches[j] + join:
				pairs.append([i, j])
	return pairs


func test_it_finds_the_pairs_in_reach_in_list_order() -> void:
	var groups := _groups([Vector2(0, 0), Vector2(30, 0), Vector2(9, 0), Vector2(-9, 1)])
	var reaches := [4.0, 4.0, 4.0, 4.0]
	assert_array(GroupPairs.near(groups, reaches, 2.0)).is_equal([[0, 2], [0, 3]])


func test_it_finds_what_trying_every_pair_finds() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 5
	for trial in 20:
		var points := []
		var reaches := []
		for _i in 30:
			points.append(Vector2(rng.randf_range(-80, 80), rng.randf_range(-80, 80)))
			reaches.append(rng.randf_range(1.0, 20.0))
		var groups := _groups(points)
		assert_array(GroupPairs.near(groups, reaches, 2.0)).is_equal(
			_every_pair(groups, reaches, 2.0)
		)
