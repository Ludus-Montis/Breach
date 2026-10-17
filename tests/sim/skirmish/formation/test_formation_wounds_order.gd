extends GdUnitTestSuite
## No outcome of the wounded hangs on list order (Decision 97): the downed and the units
## that come to, play dead, are borne or strike out alone are listed in spawn order, so
## what they do must come out the same with every list reversed - the same ids, the same
## seed. Things decided together (two coming to on one tick, two bodies a bearer could
## take) go by where they stand, then by the seeded draws, never by the list.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRecovery = preload("res://sim/skirmish/formation/formation_recovery.gd")
const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const FormationStrays = preload("res://sim/skirmish/formation/formation_strays.gd")
const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, at: Vector2, faction: String = "player") -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = faction
	_places.health(unit, 10)
	unit.max_hp = 10
	_places.pace(unit, 1.0)
	unit.fresh_speed = 1.0
	_places.stand(unit, at)
	unit.attributes = {"constitution": 10, "strength": 10}
	return unit


func _squad(squad_id: int, members: Array, tends: String = "") -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	var squad := SkirmishSquad.new(
		squad_id, "player", 1, 0.0, members.size(), typed, null, _places.store
	)
	squad.route = FormationRoute.straight(64.0)
	squad.tends = tends
	return squad


func _downed(unit: SkirmishUnit, wake_left: float = -1.0) -> SkirmishUnit:
	_places.health(unit, 0)
	UnitState.start(unit, SkirmishUnit.State.DOWNED)
	unit.wounded = 1
	_places.store.columns.put(unit.slot, UnitColumns.WAKE_LEFT, wake_left)
	return unit


func _reversed(squads: Array) -> void:
	squads.reverse()
	for squad in squads:
		squad.units.reverse()
		UnitState.regather(squad)


## Everything about the wounded, by id: hp, place, state, wounds, stamina, what it bears.
func _state(squads: Array) -> String:
	var rows := []
	for squad in squads:
		for unit in squad.units:
			var at: Vector2 = UnitPlaces.of(squad, unit).snapped(Vector2(0.0001, 0.0001))
			var wake: float = squad.store.columns.value(unit.slot, UnitColumns.WAKE_LEFT)
			var wounds := [unit.wounded, snappedf(wake, 0.0001), unit.playing_dead]
			var stamina: float = squad.store.columns.value(unit.slot, UnitColumns.STAMINA)
			var borne := [unit.carrying, unit.carried_by, snappedf(stamina, 0.0001)]
			rows.append(
				[
					unit.id,
					squad.id,
					UnitHealth.of_one(squad, unit),
					unit.state,
					UnitPlace.rank(squad, unit),
					at,
					wounds,
					borne
				]
			)
	rows.sort()
	return str(rows)


## Two downed friends behind a standing one, both coming to this tick: each takes the same
## place at its formation's back whichever is listed first.
func _two_come_to(reverse: bool) -> String:
	var standing := _unit(1, Vector2(2, 0))
	var squad := _squad(1, [standing, _downed(_unit(2, Vector2(0, 1)), 0.05)])
	squad.units.append(_downed(_unit(3, Vector2(0, -1)), 0.05))
	UnitState.regather(squad)
	var squads := [squad]
	if reverse:
		_reversed(squads)
	FormationRecovery.step(squads, 0.1, 1, 7, [])
	return _state(squads)


func test_two_who_come_to_together_take_the_same_places_whichever_is_listed_first() -> void:
	assert_str(_two_come_to(true)).is_equal(_two_come_to(false))


## Two units of a gone formation setting off home on one tick: each is the same stray
## squad, numbered the same, whichever is listed first.
func _strays_numbered(reverse: bool) -> String:
	var gone := _squad(1, [_unit(2, Vector2(0, 1)), _unit(3, Vector2(0, -1))])
	var strays := gone.units.map(func(unit): return [unit, gone])
	if reverse:
		strays.reverse()
	var squads := [gone]
	FormationStrays.adopt(strays, squads, 5, 7)
	return _state(squads)


func test_strays_setting_off_together_are_numbered_by_the_draw_not_the_list() -> void:
	assert_str(_strays_numbered(true)).is_equal(_strays_numbered(false))


## One bearer between two downed friends, and a second formation's bearer as near the
## first: who bears whom doesn't hang on which body or formation is listed first.
func _bearers(reverse: bool) -> String:
	var bodies := _squad(1, [_downed(_unit(1, Vector2(-1, 0))), _downed(_unit(2, Vector2(1, 0)))])
	var squads := [
		bodies,
		_squad(2, [_unit(3, Vector2(0, 0))], "carry"),
		_squad(3, [_unit(4, Vector2(-2, 0))], "carry"),
	]
	if reverse:
		_reversed(squads)
	FormationCarry.step(squads, 1, 7, [])
	return _state(squads)


func test_who_bears_whom_does_not_hang_on_the_list() -> void:
	assert_str(_bearers(true)).is_equal(_bearers(false))


## Two foes' bodies, each beside a taker of one formation whose leader has a messenger for
## only one of them: which goes home doesn't hang on which body or taker is listed first.
func _messenger_spent(reverse: bool) -> String:
	var bodies := _squad(1, [_downed(_unit(1, Vector2(0, 0))), _downed(_unit(2, Vector2(10, 0)))])
	for body in bodies.units:
		body.faction_id = "the_kingdom"
	bodies.faction_id = "the_kingdom"
	var leader := _unit(5, Vector2(30, 0))
	leader.leadership = 2
	leader.traits = {"messenger": 1}
	var squads := [bodies, _squad(2, [_unit(3, Vector2(1, 0)), _unit(4, Vector2(11, 0)), leader])]
	if reverse:
		_reversed(squads)
	FormationWounds.tend(squads, 1, 10, 7, [])
	return _state(squads)


func test_which_body_a_messenger_takes_home_does_not_hang_on_the_list() -> void:
	assert_str(_messenger_spent(true)).is_equal(_messenger_spent(false))


## A head-on mirror fought on past its end, its downed coming to and setting off home
## together, is the same with its lists reversed, tick for tick.
func test_a_mirror_fought_on_past_its_end_is_the_same_with_its_lists_reversed() -> void:
	var usual := _mirror(78)
	var reversed := _mirror(78)
	for tick in range(420):
		usual.step()
		_reversed(reversed.squads())
		reversed.step()
		if _state(usual.squads()) != _state(reversed.squads()):
			assert_int(tick + 1).is_equal(0)
			return


func _mirror(battle_seed: int) -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = battle_seed
	sim.blow_rolls = true
	for spawn in BattleTrials._mirror_spawns("mirror_headon", sim):
		spawn.call()
	return sim
