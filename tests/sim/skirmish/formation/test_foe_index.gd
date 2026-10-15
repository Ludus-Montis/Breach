extends GdUnitTestSuite
## One foe index a faction (FoeIndex): a squad searching it finds what it would among an
## index of its own foes alone - the squads it fights (ScrumSeek.foe_units), or those
## ScrumBlows lets it strike - with the same picks, gaps and order (Decision 97).

const FoeIndex = preload("res://sim/skirmish/formation/foe_index.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const ScrumSeek = preload("res://sim/skirmish/formation/scrum_seek.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SEED := 11
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


## A squad of `count` units standing about `centre`, scattered by a seeded generator.
func before_test() -> void:
	_places = UnitPlaces.new()


func _squad(squad_id: int, faction: String, centre: Vector2, count: int) -> SkirmishSquad:
	var rng := RandomNumberGenerator.new()
	rng.seed = squad_id
	var members: Array[SkirmishUnit] = []
	for index in range(count):
		var unit := SkirmishUnit.new()
		unit.id = squad_id * 100 + index
		unit.hp = 10
		unit.column = index
		_places.stand(unit, centre + Vector2(rng.randf_range(-4, 4), rng.randf_range(-4, 4)))
		members.append(unit)
	var squad := SkirmishSquad.new(squad_id, faction, 1, 0.0, count, members, null, _places.store)
	squad.state = SkirmishSquad.State.FIGHTING
	return squad


## Two players' squads against four enemy ones: 1 fights 3 on its front and 4 on an edge;
## 5 retreats, 6 routs.
func _field() -> Array:
	var squads := [
		_squad(1, "player", Vector2(0, 0), 6),
		_squad(2, "player", Vector2(9, 0), 5),
		_squad(3, "enemy", Vector2(2, 1), 6),
		_squad(4, "enemy", Vector2(-2, 2), 4),
		_squad(5, "enemy", Vector2(1, -2), 4),
		_squad(6, "enemy", Vector2(0, 3), 3),
	]
	squads[0].engaged_with = 3
	squads[3].flank_contacts[1] = {"foe": 1, "since": 0}
	squads[4].order = SkirmishUnit.Order.RETREAT
	squads[4].state = SkirmishSquad.State.MOVING
	squads[5].state = SkirmishSquad.State.ROUTING
	return squads


func test_the_squads_it_fights_are_its_own_foes_in_their_order() -> void:
	var squads := _field()
	var index := FoeIndex.of(squads)
	for squad in squads.slice(0, 2):
		var near := FoeIndex.fought(index, squad)
		var own := ScrumSeek.foe_units(squad, squads)
		assert_array(FoeIndex.listed(near)).is_equal(own)
		assert_bool(near["any"]).is_equal(not own.is_empty())


func test_a_search_picks_and_measures_as_its_own_foes_would() -> void:
	var squads := _field()
	var index := FoeIndex.of(squads)
	var squad: SkirmishSquad = squads[0]
	var shared := FoeIndex.fought(index, squad)
	var alone := ScrumNear.index(ScrumSeek.foe_units(squad, squads))
	for unit in squad.living():
		var mine = ScrumBlows.nearest_touching(
			squad, unit, ScrumNear.around(shared, _places.at(unit), 1.5), SEED
		)
		var theirs = ScrumBlows.nearest_touching(
			squad, unit, ScrumNear.around(alone, _places.at(unit), 1.5), SEED
		)
		assert_that(mine).is_equal(theirs)
		var gap := ScrumNear.gap_to(shared, _places.at(unit), 0.5)
		assert_float(gap).is_equal(ScrumNear.gap_to(alone, _places.at(unit), 0.5))


func test_blows_reach_every_standing_foe_or_only_a_retreat() -> void:
	var squads := _field()
	var index := FoeIndex.of(squads)
	var fighting := FoeIndex.struck(index, squads[0])
	assert_array(fighting["only"].keys()).contains_exactly_in_any_order(
		[squads[2], squads[3], squads[4]]
	)
	squads[1].state = SkirmishSquad.State.HOLDING
	var holding := FoeIndex.struck(FoeIndex.of(squads), squads[1])
	assert_array(holding["only"].keys()).contains_exactly([squads[4]])
	assert_dict(FoeIndex.struck(index, squads[5])).is_empty()  # a router strikes no one here


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
