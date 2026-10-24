extends GdUnitTestSuite
## The passes that look only near (BodyGrid) give what a look at everything gives (spec 30
## round 3, Decision 97): the bodies underfoot, checked against a plain search of every unit
## over a scattered field. The gap between two groups meeting is the crate's
## (`groups/tests.rs`), and the nearest formation one coming to joins
## (`recovery/tests.rs`) and the bearers for the downed (`carry/tests.rs`) too.

const GroundBodies = preload("res://sim/skirmish/formation/ground_bodies.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

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
		_places.health(unit, 10)
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
				UnitState.start(unit, SkirmishUnit.State.DOWNED)
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
