extends GdUnitTestSuite
## Load and stamina, per spec 28 part 7 (Decisions 117 and 120): what a unit carries
## against its strength sets its encumbrance - slower, tiring faster, dodging worse - and
## its stamina, from its constitution, is spent running and striking and regained
## otherwise; tired it is slower and less skilled, and a tired pursuer gives up the chase.

const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const FormationUnits = preload("res://sim/skirmish/formation/formation_units.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

var _places := UnitPlaces.new()


## A unit of strength 10 (carry limit 20 with the usual tuning) carrying `weight`.
func before_test() -> void:
	_places = UnitPlaces.new()


func _laden(weight: float, hauler: int = 0) -> UnitDef:
	var pack := ItemDef.new()
	pack.item_name = "pack"
	pack.weight = weight
	var unit_def := UnitDef.new()
	unit_def.hp = 10
	unit_def.speed = 1.0
	unit_def.items = [pack]
	if hauler > 0:
		unit_def.traits = {"hauler": hauler}
	return unit_def


## A unit of its own id in the shared store, at pace 1 with `stamina` of 100.
func _unit(stamina: float = 100.0) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	_places.health(unit, 10)
	_places.pace(unit, 1.0)
	unit.max_stamina = 100.0
	_places.tire(unit, stamina)
	return unit


## `unit`'s stamina, its pace this tick, in the shared store (spec 35 step 6c).
func _stamina(unit: SkirmishUnit) -> float:
	return _places.store.columns.value(unit.slot, UnitColumns.STAMINA)


func _speed(unit: SkirmishUnit) -> float:
	return _places.store.columns.value(unit.slot, UnitColumns.SPEED)


## A view of the shared store holding STAMINA.
func _view() -> UnitView:
	return UnitView.read({"store": _places.store}, [UnitColumns.STAMINA])


func _squad(units: Array) -> SkirmishSquad:
	var typed: Array[SkirmishUnit] = []
	typed.assign(units)
	return SkirmishSquad.new(1, "player", 1, 0.0, units.size(), typed, null, _places.store)


func test_its_load_against_its_strength_sets_its_encumbrance() -> void:
	assert_int(UnitArms.load_stage(_laden(10.0))).is_equal(0)
	assert_int(UnitArms.load_stage(_laden(15.0))).is_equal(1)
	assert_int(UnitArms.load_stage(_laden(30.0))).is_equal(2)
	assert_int(UnitArms.load_stage(_laden(45.0))).is_equal(3)
	assert_int(UnitArms.load_stage(_laden(45.0, 1))).is_equal(2)  # a hauler carries more


func test_a_laden_unit_is_slower_tires_faster_and_dodges_worse() -> void:
	var tuning := BattleTuning.current()
	var light := FormationUnits.make(_laden(5.0), "player", 1, 1)
	var heavy := FormationUnits.make(_laden(30.0), "player", 1, 2)

	assert_float(light.fresh_speed).is_equal(1.0)
	assert_float(heavy.fresh_speed).is_equal_approx(tuning.load_pace[2], 0.0001)
	assert_float(heavy.tiring).is_greater(light.tiring)
	assert_float(heavy.dodging).is_less(light.dodging)
	assert_float(light.max_stamina).is_equal(10 * tuning.stamina_per_constitution)


func test_running_tires_it_and_marching_light_costs_nothing() -> void:
	var tuning := BattleTuning.current()
	var runner := _unit()
	var fleeing := _squad([runner])
	fleeing.state = SkirmishSquad.State.ROUTING
	var marcher := _unit()
	var striker := _unit()

	for _i in range(10):
		FormationStamina.step([fleeing, _squad([marcher])], 0.1)
	FormationStamina.strike(_places, [[striker]])

	assert_float(_stamina(runner)).is_equal_approx(100.0 - tuning.stamina_run, 0.0001)
	assert_float(_stamina(marcher)).is_equal(100.0)
	assert_float(_speed(marcher)).is_equal(1.0)
	assert_float(_stamina(striker)).is_equal_approx(100.0 - tuning.stamina_blow, 0.0001)


func test_a_run_builds_up_and_a_pursuer_reacts_first() -> void:
	var fleeing_unit := _unit()
	var fleeing := _squad([fleeing_unit])
	fleeing.state = SkirmishSquad.State.ROUTING
	var chaser := _unit()
	var pursuing := _squad([chaser])
	pursuing.pursuit = {"returning": false}
	FormationStamina.step([fleeing, pursuing], 0.1)

	assert_float(_speed(fleeing_unit)).is_greater(1.0)  # building up
	assert_float(_speed(fleeing_unit)).is_less(fleeing_unit.run_pace)
	assert_float(_speed(chaser)).is_equal(1.0)  # still reacting
	for _i in range(30):
		FormationStamina.step([fleeing, pursuing], 0.1)
	assert_float(_speed(fleeing_unit)).is_equal_approx(fleeing_unit.run_pace, 0.0001)
	assert_float(_speed(chaser)).is_equal_approx(chaser.run_pace, 0.0001)


func test_it_recovers_only_after_a_breather_from_its_last_cost() -> void:
	var tuning := BattleTuning.current()
	var resting := _unit(50.0)
	var hardy := _unit(50.0)
	hardy.attributes = {"constitution": 20}  # half the breather, twice the rate
	var squads := [_squad([resting, hardy])]
	var delay := tuning.stamina_delay
	for _i in range(roundi(delay * 0.75 / 0.1)):
		FormationStamina.step(squads, 0.1)

	assert_float(_stamina(resting)).is_equal(50.0)  # still catching its breath
	assert_float(_stamina(hardy)).is_greater(50.0)
	for _i in range(roundi(delay / 0.1) + 10):
		FormationStamina.step(squads, 0.1)
	assert_float(_stamina(resting)).is_greater(50.0)
	FormationStamina.strike(_places, [[resting]])  # a blow restarts the breather
	var after := _stamina(resting)
	FormationStamina.step(squads, 0.1)
	assert_float(_stamina(resting)).is_equal(after)


func test_a_heavy_load_makes_marching_cost_and_a_poor_condition_slows_recovery() -> void:
	var laden := _unit()
	laden.load_stage = 2
	var marching := _squad([laden])
	var weak := _unit(50.0)
	_places.store.columns.put(weak.slot, UnitColumns.CONDITION, 0.0)  # too bad to recover
	var rested := [_squad([weak])]
	FormationStamina.step([marching], 1.0)
	for _i in range(100):
		FormationStamina.step(rested, 0.1)

	assert_float(_stamina(laden)).is_less(100.0)
	assert_float(_stamina(weak)).is_equal(50.0)


func test_who_runs() -> void:
	var hurried := _squad([_unit()])
	hurried.hurry = true
	var fighting := _squad([_unit()])
	fighting.hurry = true
	fighting.state = SkirmishSquad.State.FIGHTING
	var retreating := _squad([_unit()])
	retreating.order = SkirmishUnit.Order.RETREAT
	var leader := _unit()
	leader.leadership = 2
	leader.traits = {"retreats_at_march": 1}
	var withdrawing := _squad([leader, _unit()])
	withdrawing.order = SkirmishUnit.Order.RETREAT
	var steady := _unit()
	steady.traits = {"pursues_at_march": 1}
	var following := _squad([steady])
	following.pursuit = {"returning": false}
	var marching := _squad([_unit()])

	assert_bool(FormationStamina.runs(hurried)).is_true()
	assert_bool(FormationStamina.runs(fighting)).is_false()
	assert_bool(FormationStamina.runs(retreating)).is_true()
	assert_bool(FormationStamina.runs(withdrawing)).is_false()  # its leader keeps a march
	assert_bool(FormationStamina.runs(following)).is_false()  # its units keep a march
	assert_bool(FormationStamina.runs(marching)).is_false()


func test_a_spent_unit_cant_run() -> void:
	var spent := _unit(5.0)
	var fleeing := _squad([spent])
	fleeing.state = SkirmishSquad.State.ROUTING
	FormationStamina.step([fleeing], 0.1)

	assert_float(_speed(spent)).is_equal_approx(BattleTuning.current().stamina_pace[2], 0.0001)


func test_tired_it_is_slower_and_less_skilled() -> void:
	var tuning := BattleTuning.current()
	var fresh := _unit()
	var tired := _unit(40.0)
	var spent := _unit(5.0)
	FormationStamina.step([_squad([fresh, tired, spent])], 0.0)

	assert_float(_speed(fresh)).is_equal(1.0)
	assert_float(_speed(tired)).is_equal_approx(tuning.stamina_pace[1], 0.0001)
	assert_float(_speed(spent)).is_equal_approx(tuning.stamina_pace[2], 0.0001)
	var shift := func(u): return FormationStamina.skill_shift(u, _stamina(u))
	assert_float(shift.call(fresh)).is_equal(0.0)
	assert_float(shift.call(spent)).is_less(shift.call(tired))


func test_a_tired_pursuer_gives_up() -> void:
	var keen := _unit()
	var worn := _unit(10.0)

	assert_bool(FormationStamina.gives_up(_view(), keen)).is_false()
	assert_bool(FormationStamina.gives_up(_view(), worn)).is_true()
	assert_bool(FormationStamina.squad_gives_up(_squad([keen, worn]))).is_false()
	assert_bool(FormationStamina.squad_gives_up(_squad([worn, _unit(5.0), keen]))).is_true()
