extends GdUnitTestSuite
## The downed and the caught, per Decision 121 (spec 28 part 6), as GDScript lands what the
## crate decides (`deaths.rs`, `wounds.rs` and `wounds/pass.rs`, whose tests hold the
## rules): a unit brought to 0 hp is downed, unless past minus its constitution, which kills
## it; a body is captured by a captor, or sent home by a leader's messenger, who has one
## fewer. A routing unit struck may surrender in the fight's pass (`fight/surrender.rs`,
## test_formation_fight.gd).

const FormationWounds = preload("res://sim/skirmish/formation/formation_wounds.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")

const SECOND := 10  # ticks
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, faction: String, at: Vector2) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = faction
	_places.health(unit, 10)
	unit.max_hp = 10
	unit.dmg = 4
	_places.stand(unit, at)
	unit.attributes = {"constitution": 6}
	return unit


func _squad(squad_id: int, faction: String, members: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	return SkirmishSquad.new(squad_id, faction, 1, 0.0, members.size(), typed, null, _places.store)


## A downed kingdom unit at the origin and a player unit one cell off: [squads, body,
## taker].
func _scene() -> Array:
	var body := _unit(1, "the_kingdom", Vector2.ZERO)
	_places.health(body, 0)
	UnitState.start(body, SkirmishUnit.State.DOWNED)
	var taker := _unit(2, "player", Vector2(1, 0))
	return [[_squad(1, "the_kingdom", [body]), _squad(2, "player", [taker])], body, taker]


func _types(events: Array) -> Array:
	return events.map(func(e): return e["type"])


func test_brought_to_zero_it_is_downed_and_past_its_constitution_it_dies() -> void:
	var downed := _unit(1, "player", Vector2.ZERO)
	_places.health(downed, -5)  # within its constitution of 6: the excess is lost
	var killed := _unit(2, "player", Vector2(1, 0))
	_places.health(killed, -6)
	var events := []

	FormationDeaths.bury([_squad(1, "player", [downed, killed])], 1, events)

	assert_int(downed.state).is_equal(SkirmishUnit.State.DOWNED)
	assert_int(UnitHealth.of_one(_places, downed)).is_equal(0)
	assert_int(killed.state).is_equal(SkirmishUnit.State.DEAD)
	assert_array(_types(events)).contains(["downed", "died"])
	assert_bool(downed.is_alive()).is_false()


func test_a_captor_captures_it_instead() -> void:
	var scene := _scene()
	scene[2].traits = {"captor": 1}
	var events := []

	FormationWounds.tend(scene[0], 1, SECOND, 1, events)

	assert_int(scene[1].state).is_equal(SkirmishUnit.State.TAKEN)
	assert_array(_types(events)).contains(["captured"])


func test_a_leader_with_a_messenger_sends_one_home_to_shake_its_side() -> void:
	var scene := _scene()
	var leader := _unit(4, "player", Vector2(30, 0))
	leader.leadership = 2
	leader.traits = {"messenger": 1}
	_places.store.admit(leader)  # held in the crate, as a squad's units are
	scene[0][1].units.append(leader)
	UnitState.regather(scene[0][1])
	var events := []

	FormationWounds.tend(scene[0], 1, SECOND, 1, events)

	assert_int(scene[1].state).is_equal(SkirmishUnit.State.RELEASED)
	assert_int(leader.traits["messenger"]).is_equal(0)
	assert_array(_types(events)).contains(["sent_home"])


func test_the_crate_pass_captures_the_body_and_tells_of_it() -> void:
	var scene := _scene()
	scene[2].traits = {"captor": 1}
	var events := []

	FormationWounds.tend(scene[0], 1, SECOND, 1, events)

	var captured := {"type": "captured", "tick": 1, "squad": 1, "faction": "the_kingdom"}
	captured.merge({"unit": 1, "by": 2})
	assert_array(events.filter(func(e): return e["type"] == "captured")).is_equal([captured])
	assert_int(_places.store.columns.value(scene[1].slot, UnitColumns.STATE)).is_equal(
		SkirmishUnit.State.TAKEN
	)
