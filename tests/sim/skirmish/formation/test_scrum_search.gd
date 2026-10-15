extends GdUnitTestSuite
## The scrum's searches through grids (spec 30 round 3) choose exactly as looking through
## every body, foe and slot does: on a crowded, jittered field, SlotSearch picks the slot
## ScrumSlots.pick picks, ScrumNear gives ScrumSlots.gap_to's gap, and UnitSteer steers the
## same with its crowd as without, for seeker after seeker.

const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const SlotSearch = preload("res://sim/skirmish/formation/slot_search.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SEED := 41
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


## [seekers' squad, foes' squad]: two jittered blocks of `count` grems (and a brute or two)
## pressed against each other, the seekers west of the foes.
func before_test() -> void:
	_places = UnitPlaces.new()


func _field(count: int) -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var sides := []
	for side in range(2):
		var members: Array[SkirmishUnit] = []
		for index in range(count):
			var unit := SkirmishUnit.new()
			unit.id = side * 1000 + index + 1
			unit.squad_id = side + 1
			unit.footprint_width = 2 if index % 17 == 5 else 1
			unit.footprint_depth = unit.footprint_width
			var column := index % 8
			var rank := index / 8
			var x := 20.0 - rank * 1.1 if side == 0 else 21.2 + rank * 1.1
			_places.stand(unit, Vector2(x, 10.0 + column * 1.1) + _jitter(rng))
			unit.bearing = 90.0 if side == 0 else 270.0 + rng.randf_range(-40.0, 40.0)
			members.append(unit)
		var faction := "player" if side == 0 else "the_kingdom"
		sides.append(SkirmishSquad.new(side + 1, faction, 1, 0.0, 8, members, null, _places.store))
	return sides


func _jitter(rng: RandomNumberGenerator) -> Vector2:
	return Vector2(rng.randf_range(-0.4, 0.4), rng.randf_range(-0.4, 0.4))


func _foes(squad: SkirmishSquad) -> Array:
	return squad.living().map(func(unit): return [unit, squad])


func _named(slot: Array) -> Array:
	return [] if slot.is_empty() else [slot[1].id, slot[3]]


func test_slot_search_picks_the_slot_scrum_slots_picks() -> void:
	var sides := _field(48)
	var bodies := ScrumSlots.bodies(sides)
	var crowd := BodyGrid.of_bodies(bodies)
	var slots := ScrumSlots.round_foes(_foes(sides[1]), 0.5)
	var slot_ring := SlotSearch.ring(_foes(sides[1]), 0.5, bodies, crowd)
	var claimed := []
	var claims := {}
	var picked := 0
	for seeker in sides[0].living().filter(func(u): return u.footprint_width == 1):
		var place: Vector2 = _places.at(seeker) + Vector2(-0.3, 0.2)
		var ground := [place, bodies, claimed, null, SEED]
		var indexed := ground + [crowd, claims]
		for crowded in [false, true]:
			var whole := ScrumSlots.pick(seeker, _places.at(seeker), slots, ground, crowded)
			var found := SlotSearch.pick(seeker, _places.at(seeker), slot_ring, indexed, crowded)
			assert_array(_named(found)).is_equal(_named(whole))
		var taken := ScrumSlots.pick(seeker, _places.at(seeker), slots, ground)
		if not taken.is_empty():  # it claims the slot, as a seeker does
			picked += 1
			claimed.append(taken[0])
			SlotSearch.claim(claims, taken[0])
	assert_int(picked).is_greater(3)


func test_scrum_near_gives_the_gap_to_the_nearest_foe() -> void:
	var sides := _field(48)
	var foes := _foes(sides[1])
	var near := ScrumNear.index(foes)
	for unit in sides[0].living():
		var radius: float = 0.5 * unit.footprint_width
		var gap := ScrumNear.gap_to(near, _places.at(unit), radius)
		assert_float(gap).is_equal(ScrumSlots.gap_to(_places.at(unit), radius, foes))
	var far := Vector2(-40, 3)
	assert_float(ScrumNear.gap_to(near, far, 0.5)).is_equal(ScrumSlots.gap_to(far, 0.5, foes))


func test_steering_through_its_crowd_steers_as_through_every_body() -> void:
	var sides := _field(48)
	var bodies := ScrumSlots.bodies(sides)
	var crowd := BodyGrid.of_bodies(bodies)
	for unit in sides[0].living():
		for goal in [Vector2(25, 14), Vector2(19, 4), _places.at(unit) + Vector2(0.4, 0.1)]:
			var whole := UnitSteer.toward(unit, _places.at(unit), goal, bodies, SEED)
			var through := UnitSteer.toward(unit, _places.at(unit), goal, bodies, SEED, crowd)
			assert_vector(through).is_equal(whole)


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
