extends GdUnitTestSuite
## FormationCarry, per Decision 126: a formation told to tend its wounded picks up its
## downed friends - "carry" bears them on, slowed by their weight; "recover" sends the
## bearer home with them, both returning to the reserve; a bearer that falls drops its
## body. Who picks up whom is the crate's, tested in `carry/tests.rs`.

const FormationCarry = preload("res://sim/skirmish/formation/formation_carry.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
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
	_places.pace(unit, 1.0)
	unit.fresh_speed = 1.0
	_places.stand(unit, at)
	unit.attributes = {"strength": 10}
	return unit


func _squad(squad_id: int, faction: String, members: Array, tends: String = "") -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(members)
	var squad := SkirmishSquad.new(
		squad_id, faction, 1, 0.0, members.size(), typed, null, _places.store
	)
	squad.tends = tends
	return squad


## A downed player unit at the origin, its friend a cell off in a formation that `tends`.
func _scene(tends: String) -> Array:
	var body := _unit(1, "player", Vector2.ZERO)
	_places.health(body, 0)
	UnitState.start(body, SkirmishUnit.State.DOWNED)
	var bearer := _unit(2, "player", Vector2(1, 0))
	return [[_squad(1, "player", [body]), _squad(2, "player", [bearer], tends)], body, bearer]


func test_a_formation_that_carries_bears_its_downed_on_slowed_by_them() -> void:
	var scene := _scene("carry")
	var events := []
	var strays := FormationCarry.step(scene[0], 1, 1, events)

	assert_int(scene[1].state).is_equal(SkirmishUnit.State.CARRIED)
	assert_int(scene[2].carrying).is_equal(1)
	assert_float(scene[2].fresh_speed).is_less(1.0)  # its load
	assert_array(strays).is_empty()
	_places.stand(scene[2], Vector2(5, 5))
	FormationCarry.step(scene[0], 2, 1, [])
	assert_vector(_places.at(scene[1])).is_equal(Vector2(5, 5))


func test_one_that_recovers_sends_the_bearer_home_with_it() -> void:
	var scene := _scene("recover")
	var strays := FormationCarry.step(scene[0], 1, 1, [])

	assert_int(strays.size()).is_equal(1)
	assert_object(strays[0][0]).is_same(scene[2])
	var home := {"type": "fled_home", "unit": 2}
	var events := [home]
	FormationCarry.step(scene[0], 2, 1, events)
	assert_bool(events.any(func(e): return e["type"] == "fled_home" and e["unit"] == 1)).is_true()


func test_a_bearer_that_falls_drops_its_body() -> void:
	var scene := _scene("carry")
	FormationCarry.step(scene[0], 1, 1, [])
	_places.stand(scene[2], Vector2(4, 0))
	UnitState.put(scene[0][1], scene[2], SkirmishUnit.State.DOWNED)
	FormationCarry.step(scene[0], 2, 1, [])

	assert_int(scene[1].state).is_equal(SkirmishUnit.State.DOWNED)
	assert_int(scene[1].carried_by).is_equal(0)


func test_the_pass_is_the_crates_and_what_it_did_is_gdscripts_too() -> void:
	var scene := _scene("recover")
	var events := []
	FormationCarry.step(scene[0], 1, 1, events)
	var field: Object = _places.store.field()
	var names: PackedStringArray = field.sheet_names()
	var sheet: PackedFloat64Array = field.sheet(scene[2].slot)
	var bearer: SkirmishUnit = scene[2]

	var told := events.map(func(e): return [e["type"], e["unit"], e["body"]])
	assert_array(told).is_equal([["borne", 2, 1]])
	var mine := [
		float(bearer.carrying), float(bearer.load_stage), bearer.fresh_speed, bearer.tiring
	]
	var crate := ["carrying", "load_stage", "fresh_speed", "tiring"].map(
		func(name): return sheet[names.find(name)]
	)
	assert_array(mine).is_equal(crate)
	assert_array([scene[1].carried_by, scene[1].state]).is_equal([2, SkirmishUnit.State.CARRIED])
	FormationCarry.step(scene[0], 2, 1, [{"type": "fled_home", "unit": 2}])
	assert_array(scene[0][0].units).is_empty()  # the body came home with its bearer
