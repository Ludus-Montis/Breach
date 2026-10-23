extends GdUnitTestSuite
## The nearest foe a unit touches (ScrumBlows.nearest_touching, which ScrumRegroup reads
## until it ports, spec 39 step 4c): of foes as near and as much in its front, the seeded
## draw picks, whatever the list's order (Decision 97). The scrum's own picks are the
## crate's (blows.rs, facing.rs; their cargo tests).

const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SEED := 11
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


## A squad of `count` units standing about `centre`, scattered by a seeded generator.
func _squad(squad_id: int, faction: String, centre: Vector2, count: int) -> SkirmishSquad:
	var rng := RandomNumberGenerator.new()
	rng.seed = squad_id
	var members: Array[SkirmishUnit] = []
	for index in range(count):
		var unit := SkirmishUnit.new()
		unit.id = squad_id * 100 + index
		_places.health(unit, 10)
		_places.set_place(unit, Vector2i(0, index))
		_places.stand(unit, centre + Vector2(rng.randf_range(-4, 4), rng.randf_range(-4, 4)))
		members.append(unit)
	var squad := SkirmishSquad.new(squad_id, faction, 1, 0.0, count, members, null, _places.store)
	squad.state = SkirmishSquad.State.FIGHTING
	return squad


func test_foes_at_the_same_distance_go_by_the_seeded_draw() -> void:
	var squad := _squad(1, "player", Vector2.ZERO, 1)
	var foes := _squad(2, "enemy", Vector2.ZERO, 2)
	var unit: SkirmishUnit = squad.units[0]
	_places.stand(unit, Vector2.ZERO)
	_places.face(unit, 90.0)
	_places.stand(foes.units[0], Vector2(0.5, 1.0))  # both touching, as far, in front
	_places.stand(foes.units[1], Vector2(0.5, -1.0))
	var entries := foes.living().map(func(foe): return [foe, foes])
	var first: SkirmishUnit = foes.units[0]
	if ScrumContest.draw(foes.units[1], SEED) < ScrumContest.draw(first, SEED):
		first = foes.units[1]
	assert_that(ScrumBlows.nearest_touching(squad, unit, entries, SEED)).is_equal(_places.at(first))
	entries.reverse()
	assert_that(ScrumBlows.nearest_touching(squad, unit, entries, SEED)).is_equal(_places.at(first))
