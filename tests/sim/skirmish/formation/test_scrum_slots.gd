extends GdUnitTestSuite
## Slots, per Decisions 88 and 106 and spec 30 round 1: round a foe's body, as many
## touching slots as fit (6 for a grem round a grem), laid out from the foe's own front; a
## slot is not open under another body or a nearer seeker's claim, nor beyond the leash; and
## a seeker takes the nearest open one, ties going by its own frame.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, at: Vector2, bearing: float, size: int = 1) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.stand(unit, at)
	_places.face(unit, bearing)
	unit.footprint_width = size
	unit.footprint_depth = size
	return unit


func _foes(foe: SkirmishUnit) -> Array:
	var squad := SkirmishSquad.new(
		9, "the_kingdom", -1, 0.0, 1, [foe] as Array[SkirmishUnit], null, _places.store
	)
	return [[foe, squad]]


func test_six_grems_fit_round_a_grem_from_its_front() -> void:
	var foe := _unit(1, Vector2(10, 10), 90.0)  # facing east

	var slots := ScrumSlots.round_foes(_foes(foe), 0.5)

	assert_int(slots.size()).is_equal(6)
	assert_vector(slots[0][0]).is_equal_approx(Vector2(11, 10), Vector2(0.0001, 0.0001))
	assert_int(ScrumSlots.round_foes(_foes(_unit(2, Vector2.ZERO, 0.0, 2)), 0.5).size()).is_equal(9)


func test_a_slot_under_a_body_or_a_claim_or_past_the_leash_is_not_open() -> void:
	var seeker := _unit(5, Vector2(14, 10), 270.0)
	var point := Vector2(11, 10)
	var place := Vector2(14, 10)

	assert_bool(ScrumSlots.open(seeker, point, [place, [], [], null, 0])).is_true()
	var body := [[Vector2(11.4, 10), 0.5, _unit(7, Vector2(11.4, 10), 0.0)]]
	assert_bool(ScrumSlots.open(seeker, point, [place, body, [], null, 0])).is_false()
	(
		assert_bool(ScrumSlots.open(seeker, point, [place, [], [Vector2(11, 10.6)], null, 0]))
		. is_false()
	)
	var far := place + Vector2(BattleTuning.current().scrum_leash + 1.0, 0)
	assert_bool(ScrumSlots.open(seeker, far, [place, [], [], null, 0])).is_false()


func test_a_seeker_takes_the_nearest_open_slot() -> void:
	var foe := _unit(1, Vector2(10, 10), 90.0)
	var seeker := _unit(5, Vector2(13, 10), 270.0)  # east of it, facing it
	var slots := ScrumSlots.round_foes(_foes(foe), 0.5)

	var picked := ScrumSlots.pick(
		seeker,
		_places.at(seeker),
		_places.facing(seeker),
		slots,
		[_places.at(seeker), [], [], null, 3]
	)

	assert_vector(picked[0]).is_equal_approx(Vector2(11, 10), Vector2(0.0001, 0.0001))
	var claimed := [Vector2(11, 10)]
	var next := ScrumSlots.pick(
		seeker,
		_places.at(seeker),
		_places.facing(seeker),
		slots,
		[_places.at(seeker), [], claimed, null, 3]
	)
	assert_float(next[0].distance_to(Vector2(10, 10))).is_equal_approx(1.0, 0.0001)
	assert_float(next[0].x).is_greater(10.0)  # one of the two to the east side next


func test_with_no_slot_open_the_crowded_pick_is_the_nearest_taken_one() -> void:
	var foe := _unit(1, Vector2(10, 10), 90.0)
	var seeker := _unit(5, Vector2(13, 10), 270.0)
	var slots := ScrumSlots.round_foes(_foes(foe), 0.5)
	var claimed := slots.map(func(s): return s[0])  # every slot taken
	var ground := [_places.at(seeker), [], claimed, null, 3]

	(
		assert_array(
			ScrumSlots.pick(seeker, _places.at(seeker), _places.facing(seeker), slots, ground)
		)
		. is_empty()
	)
	var nearest := ScrumSlots.pick(
		seeker, _places.at(seeker), _places.facing(seeker), slots, ground, true
	)
	assert_vector(nearest[0]).is_equal_approx(Vector2(11, 10), Vector2(0.0001, 0.0001))
