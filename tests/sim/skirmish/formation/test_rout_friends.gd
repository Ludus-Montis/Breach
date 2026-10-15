extends GdUnitTestSuite
## A rout's searches through grids (RoutFriends, ScrumNear.nearest; spec 30 round 3)
## choose exactly as looking through every unit did: on a field of many friendly
## formations - some routing, some standing, some led - and an enemy, laid out on whole and
## half cells so that many units stand equally near, each search gives what the search of
## every unit in the list's order gives (Decision 97), for router after router, and still
## once some friends have fallen.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const RoutCatch = preload("res://sim/skirmish/formation/rout_catch.gd")
const RoutFriends = preload("res://sim/skirmish/formation/rout_friends.gd")

const SEED := 23
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


## Eight player formations (every third routing, every other led) and a kingdom one, on a
## bent route, their units on a half-cell lattice, some loose a little off their places.
func before_test() -> void:
	_places = UnitPlaces.new()


func _field() -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = 5
	var route := FormationRoute.new(
		PackedVector2Array([Vector2(0, 0), Vector2(40, 0), Vector2(70, 18)])
	)
	var squads := []
	for index in range(9):
		var members: Array[SkirmishUnit] = []
		for slot in range(24):
			var unit := SkirmishUnit.new()
			unit.id = index * 100 + slot + 1
			unit.squad_id = index + 1
			unit.footprint_width = 2 if slot % 11 == 3 else 1
			unit.footprint_depth = unit.footprint_width
			unit.leadership = 2 if index % 2 == 0 and slot == 0 else 0
			var x := 4.0 + index * 6.0 + (slot / 6) * 1.5 + rng.randi_range(0, 2) * 0.5
			_places.stand(unit, Vector2(x, (slot % 6) * 1.5 - 4.0 + rng.randi_range(0, 2) * 0.5))
			members.append(unit)
		var faction := "the_kingdom" if index == 8 else "player"
		var squad := SkirmishSquad.new(index + 1, faction, 1, 0.0, 6, members, null, _places.store)
		squad.route = route
		squad.state = SkirmishSquad.State.ROUTING if index % 3 == 1 else SkirmishSquad.State.HOLDING
		for unit in members.slice(0, 4):  # a few loose a little off their places
			var at: Vector2 = _places.at(unit) + Vector2(rng.randi_range(-1, 1), 0.5)
			SquadEntries.loosen(squad, unit, at)
		squads.append(squad)
	return squads


## Points routers stand at: on, between and beside units, a lattice of quarter cells.
func _points() -> Array:
	var out := []
	for x in range(0, 120):
		for y in range(-12, 12, 3):
			out.append(Vector2(x * 0.5 + 0.25 * (y % 2), y * 0.5))
	return out


func test_crushing_finds_the_friend_every_unit_would() -> void:
	var squads := _field()
	var friends := {"squads": squads}
	for router_squad in squads.filter(func(s): return s.state == SkirmishSquad.State.ROUTING):
		for at in _points():
			var whole := _crush_all(router_squad, squads, at)
			var found := RoutFriends.crushed(friends, router_squad, at, SEED)
			assert_array(_ids(found)).is_equal(_ids(whole))
	_fell(squads)
	for at in _points():
		var whole := _crush_all(squads[1], squads, at)
		assert_array(_ids(RoutFriends.crushed(friends, squads[1], at, SEED))).is_equal(_ids(whole))


func test_catching_finds_the_friend_every_unit_would() -> void:
	var squads := _field()
	var friends := {"squads": squads}
	var hits := 0
	for at in _points():
		for reach in [1.0, 4.0]:
			for led in [false, true]:
				var whole = _friend_all(squads[4], at, squads, reach, led)
				var found := RoutCatch.friend_near(squads[4], at, friends, reach, led, SEED)
				assert_object(found).is_same(whole)
				hits += 0 if whole == null else 1
	assert_int(hits).is_greater(100)


func test_refuge_and_turning_aside_are_as_every_unit_gives() -> void:
	var squads := _field()
	var friends := {"squads": squads}
	var router: SkirmishSquad = squads[7]
	var refuges := 0
	for at in _points():
		var along := [router.route.distance_of(at), 0.0, SEED]
		var whole = _refuge_all(router, at, along, squads)
		assert_object(RoutFriends.refuge(friends, router, at, along)).is_same(whole)
		if whole != null:
			refuges += 1
			for side in [Vector2(0, 1), router.route.heading_at(along[0]).orthogonal()]:
				var short := RoutFriends.short_of(friends, whole, at, side)
				assert_float(short).is_equal(_short_of_all(whole, at, side))
	assert_int(refuges).is_greater(100)
	_fell(squads)
	for at in _points():
		var along := [router.route.distance_of(at), 0.0, SEED]
		assert_object(RoutFriends.refuge(friends, router, at, along)).is_same(
			_refuge_all(router, at, along, squads)
		)


func test_an_enemy_near_and_the_nearest_unit_are_as_every_unit_gives() -> void:
	var squads := _field()
	var friends := {"squads": squads}
	var near := 0
	for at in _points():
		var whole := _enemy_near_all(squads[7], squads, [at])
		assert_bool(RoutFriends.enemy_near(friends, squads[7], [at])).is_equal(whole)
		near += 1 if whole else 0
		var index := ScrumNear.index(squads[3].living().map(func(u): return [u, squads[3]]))
		var found := ScrumNear.nearest(index, at, SEED)
		assert_object(index["foes"][found][0]).is_same(_nearest_all(at, squads[3]))
	assert_int(near).is_greater(10)


func _ids(hit: Array) -> Array:
	return [] if hit.is_empty() else [hit[0].id, hit[1].id]


## Some of every formation's units fall.
func _fell(squads: Array) -> void:
	for squad in squads:
		for unit in squad.units.slice(0, 24, 5):
			unit.state = SkirmishUnit.State.DEAD


# The searches of every unit, as they were before the grids.


func _crush_all(squad: SkirmishSquad, squads: Array, at: Vector2) -> Array:
	var hit := []
	for friend in squads:
		if friend == squad or friend.faction_id != squad.faction_id:
			continue
		if friend.state == SkirmishSquad.State.ROUTING:
			continue
		for unit in friend.living():
			var gap: float = _places.at(unit).distance_to(at)
			var key := [snappedf(gap, 0.000001), ScrumContest.draw(unit, SEED)]
			if gap < 1.0 and (hit.is_empty() or key < hit[0]):
				hit = [key, friend, unit]
	return [] if hit.is_empty() else [hit[1], hit[2]]


func _friend_all(squad: SkirmishSquad, at: Vector2, squads: Array, reach: float, led: bool):
	var best: SkirmishSquad = null
	var best_key := []
	for friend in squads:
		if friend == squad or friend.faction_id != squad.faction_id or not RoutCatch.stands(friend):
			continue
		if led and FormationMorale.leadership(friend) < 1:
			continue
		for unit in friend.living():
			var there := ScrumReach.at(friend, unit)
			var gap := there.distance_to(at) - ScrumReach.radius(unit)
			var key := [snappedf(gap, 0.000001), ScrumContest.draw(unit, SEED)]
			if (
				gap <= reach
				and RoutCatch.behind(squad, at, there)
				and (best == null or key < best_key)
			):
				best = friend
				best_key = key
	return best


func _refuge_all(squad: SkirmishSquad, at: Vector2, along: Array, squads: Array):
	var best = null
	var best_key := []
	for friend in squads:
		if friend == squad or friend.faction_id != squad.faction_id or not RoutCatch.stands(friend):
			continue
		for unit in friend.living():
			var there := ScrumReach.at(friend, unit)
			var ahead: bool = (
				(squad.route.distance_of(there) - along[0]) * (along[1] - along[0]) > 0.0
			)
			var key := [
				snappedf(there.distance_to(at), 0.000001), ScrumContest.draw(unit, along[2])
			]
			if ahead and (best == null or key < best_key):
				best = friend
				best_key = key
	return best


func _short_of_all(friend: SkirmishSquad, at: Vector2, side: Vector2) -> float:
	var across: Array = friend.living().map(
		func(u): return (ScrumReach.at(friend, u) - at).dot(side)
	)
	return clampf(0.0, across.min() - 0.5, across.max() + 0.5)


func _enemy_near_all(squad: SkirmishSquad, squads: Array, points: Array) -> bool:
	for other in squads:
		if other.faction_id == squad.faction_id or other.is_destroyed():
			continue
		for enemy in other.living():
			for at in points:
				if _places.at(enemy).distance_to(at) <= BattleTuning.current().rout_enemy_near:
					return true
	return false


func _nearest_all(at: Vector2, squad: SkirmishSquad) -> SkirmishUnit:
	var best: SkirmishUnit = null
	var best_key := []
	for unit in squad.living():
		var there := ScrumReach.at(squad, unit)
		var key := [snappedf(there.distance_to(at), 0.000001), ScrumContest.draw(unit, SEED)]
		if best_key.is_empty() or key < best_key:
			best_key = key
			best = unit
	return best
