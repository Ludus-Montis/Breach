extends GdUnitTestSuite
## No world direction and no handedness (spec 30 round 1, agenda 5): whenever two slots are
## equally good the tie goes by the unit's own frame, then a seeded draw - never a world
## direction, a scan order or always the same hand. A seeker with two slots mirror images
## of each other about its own line of advance takes either about as often across battles.
## (Exact mirror runs can't match: a seeker facing a choice that is itself mirror-symmetric
## must pick a hand, and a reflection flips it.)

const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, at: Vector2, bearing: float) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	_places.stand(unit, at)
	_places.face(unit, bearing)
	return unit


## Which side the seeker heading south onto a foe facing north takes, its front slot
## claimed: 1 for the foe's right (east), -1 for its left.
func _side(battle_seed: int) -> int:
	var foe := _unit(1, Vector2(10, 12), 0.0)
	var squad := SkirmishSquad.new(
		9, "the_kingdom", -1, 0.0, 1, [foe] as Array[SkirmishUnit], null, _places.store
	)
	var seeker := _unit(5, Vector2(10, 9), 180.0)
	var slots := ScrumSlots.round_foes([[foe, squad]], 0.5)
	var claimed := [Vector2(10, 11)]  # the slot dead ahead of the foe, towards the seeker
	var ground := [_places.at(seeker), [], claimed, null, battle_seed]
	var picked := ScrumSlots.pick(seeker, _places.at(seeker), _places.facing(seeker), slots, ground)
	return 1 if picked[0].x > 10.0 else -1


func test_two_mirror_image_slots_are_taken_about_as_often() -> void:
	var right := 0
	for battle_seed in range(1, 61):
		right += 1 if _side(battle_seed) > 0 else 0

	assert_int(right).is_between(20, 40)  # 30 expected; sd about 4
