extends GdUnitTestSuite
## Units front first by place (PlaceOrder): the order a comparison of ranks then columns
## sorts them into, a place held twice included.

const PlaceOrder = preload("res://sim/skirmish/formation/place_order.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _units(places: Array) -> Array:
	var out := []
	for index in range(places.size()):
		var unit := SkirmishUnit.new()
		unit.id = index + 1
		_places.stand(unit, Vector2.ZERO)
		_places.set_place(unit, places[index])
		out.append(unit)
	return out


func _compared(units: Array) -> Array:
	var out := units.duplicate()
	out.sort_custom(
		func(a, b):
			var one := UnitPlace.of(_places, a)
			var other := UnitPlace.of(_places, b)
			return one.x < other.x or (one.x == other.x and one.y < other.y)
	)
	return out


func test_front_rank_first_and_each_rank_left_to_right() -> void:
	var units := _units([Vector2i(2, 0), Vector2i(0, 3), Vector2i(1, -2), Vector2i(0, -1)])

	var ordered := _front_first(units)

	assert_array(ordered.map(func(u): return u.id)).contains_exactly([4, 2, 3, 1])


func test_many_places_sort_as_the_comparison_does() -> void:
	var places := []
	for rank in range(40):
		for column in range(-8, 24):
			places.append(Vector2i(rank, column))
	seed(5)
	places.shuffle()
	var units := _units(places)

	assert_array(_front_first(units)).is_equal(_compared(units))


func test_a_place_held_twice_sorts_as_the_comparison_does() -> void:
	var units := _units([Vector2i(1, 1), Vector2i(0, 0), Vector2i(1, 1), Vector2i(0, 2)])

	assert_array(_front_first(units)).is_equal(_compared(units))


func _front_first(units: Array) -> Array:
	var view := UnitView.read(_places, UnitView.PLACES)
	return PlaceOrder.front_first(units, view.rank, view.column)
