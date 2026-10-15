extends GdUnitTestSuite
## The passes that look only near (BodyGrid) give what a look at everything gives (spec 30
## round 3, Decision 97): the bodies underfoot, the bearers for the downed, the nearest
## formation one coming to joins, and the gap between two groups meeting - each checked
## against a plain search of every unit over a scattered field.

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationGroups = preload("res://sim/skirmish/formation/formation_groups.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SEED := 11
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


## `count` units of `faction` scattered over a field 40 cells square, grems and brutes.
func before_test() -> void:
	_places = UnitPlaces.new()


func _scatter(rng: RandomNumberGenerator, count: int, faction: String, first: int) -> Array:
	var out := []
	for index in count:
		var unit := SkirmishUnit.new()
		unit.id = first + index
		unit.faction_id = faction
		unit.hp = 10
		_places.stand(unit, Vector2(rng.randf_range(0, 40), rng.randf_range(0, 40)))
		var size := 2 if rng.randf() < 0.2 else 1
		unit.footprint_width = size
		unit.footprint_depth = size
		out.append(unit)
	return out


func _squad(squad_id: int, faction: String, members: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	var squad := SkirmishSquad.new(
		squad_id, faction, 1, 0.0, members.size(), typed, null, _places.store
	)
	squad.tends = "carry"
	squad.state = SkirmishSquad.State.HOLDING
	return squad


func _field() -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = 5
	var squads := []
	for squad_id in range(1, 7):
		var faction := "player" if squad_id % 2 == 1 else "the_kingdom"
		var members := _scatter(rng, 40, faction, squad_id * 100)
		for unit in members:
			if rng.randf() < 0.3:
				unit.state = SkirmishUnit.State.DOWNED
		squads.append(_squad(squad_id, faction, members))
	return squads


func test_the_bodies_underfoot_are_those_a_look_at_all_finds() -> void:
	var squads := _field()
	var ground := GroundBodies.ground(squads)
	var tuning := BattleTuning.current()
	for squad in squads:
		for walker in squad.living():
			var step: Vector2 = _places.at(walker) + Vector2(0.5, 0)
			var worst := 1.0
			for body in ground["lying"]:
				var reach := ScrumReach.radius(walker) + ScrumReach.radius(body)
				if _places.at(body).distance_to(step) >= reach:
					continue
				var weight: float = body.footprint_width * body.footprint_depth
				weight /= float(walker.footprint_width * walker.footprint_depth)
				worst = minf(worst, 1.0 / (1.0 + weight * tuning.bodies_ground_drag))
				if weight >= tuning.bodies_ground_block:
					worst = 0.0
					break
			assert_float(GroundBodies.underfoot(walker, step, ground)).is_equal(worst)


func test_the_bearers_paired_are_those_a_look_at_all_finds() -> void:
	var squads := _field()
	var units := {}
	for squad in squads:
		for unit in squad.units:
			units[unit.id] = [unit, squad]
	var tuning := BattleTuning.current()
	var expected := []
	for squad in squads:
		for unit_id in units:
			var body: SkirmishUnit = units[unit_id][0]
			if body.state != SkirmishUnit.State.DOWNED or body.faction_id != squad.faction_id:
				continue
			var guarded := units.values().any(
				func(entry):
					return (
						entry[0].is_alive()
						and entry[0].faction_id != body.faction_id
						and (
							_places.at(entry[0]).distance_to(_places.at(body))
							<= tuning.wounds_guard_reach
						)
					)
			)
			for unit in [] if guarded else squad.living():
				var gap: float = _places.at(unit).distance_to(_places.at(body))
				if gap <= tuning.wounds_reach and FormationCarry._stage(unit, body) < 3:
					expected.append([body.id, unit.id])
	var pairs := FormationCarry._pairs(squads, units, SEED)
	var got := pairs.map(func(pair): return [pair[1].id, pair[2].id])
	assert_array(expected).is_not_empty()
	assert_array(got).is_equal(expected)


func test_one_coming_to_joins_the_formation_a_look_at_all_finds() -> void:
	var squads := _field()
	var friends := FormationRecovery._index(squads, true, SEED)
	var joined := {true: 0, false: 0}  # some find one, some none in sight
	for squad in squads:
		for unit in squad.units:
			if unit.state != SkirmishUnit.State.DOWNED:
				continue
			unit.detection = [0.5, 3.0, 40.0][unit.id % 3]
			var best = null
			var best_key := [INF]
			for other_squad in squads:
				if other_squad.faction_id != unit.faction_id:
					continue
				for other in other_squad.living():
					var gap: float = _places.at(other).distance_to(_places.at(unit))
					var key := [snappedf(gap, 0.000001), ScrumContest.squad_draw(other_squad, SEED)]
					if gap <= unit.detection and (best == null or key < best_key):
						best = other_squad
						best_key = key
			var pick := FormationRecovery._nearest(unit, friends)
			assert_object(pick[0]).is_same(best)
			assert_float(pick[1]).is_equal(best_key[0])
			joined[best != null] += 1
	assert_int(joined[true]).is_greater(0)
	assert_int(joined[false]).is_greater(0)


func test_the_gap_between_groups_is_exact_within_group_join() -> void:
	var join := BattleTuning.current().group_join
	var rng := RandomNumberGenerator.new()
	rng.seed = 9
	var within := 0
	for trial in 20:
		var one := _squad(1, "player", _scatter(rng, 30 if trial % 2 == 0 else 8, "player", 1))
		var other := _squad(2, "player", _scatter(rng, 30, "player", 100))
		for unit in other.units:
			_places.move(unit, _places.at(unit) + Vector2(trial * 2.0, 0))
		one.position = Vector2(20, 20)
		other.position = Vector2(20 + trial * 2.0, 20)
		one.width = 40
		other.width = 40
		var least := INF
		for unit in one.living():
			for friend in other.living():
				var apart: float = _places.at(unit).distance_to(_places.at(friend))
				least = minf(least, apart - ScrumReach.radius(unit) - ScrumReach.radius(friend))
		least = maxf(least, 0.0)
		var gap := FormationGroups._gap(one, other, {})
		if least <= join:
			assert_float(gap).is_equal(least)
			within += 1
		else:
			assert_float(gap).is_greater(join)
	assert_int(within).is_between(1, 19)
