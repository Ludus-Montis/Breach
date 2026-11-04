extends GdUnitTestSuite
## The field's waves played in the crate (specs/39-simulation-in-rust.md step 8c-viii,
## native field_play.rs and waves.rs): `BodyField.field_play` sets FormationField up, and
## plays it tick by tick as FormationField.step does (the builders filling the waves, the
## waves announcing, the battle's tick, the player's routers home banked), each wave sent on
## the ticks it is given as FormationField.send sends it. The builders and waves tell of as
## much, and the crate's whole state is then the one GDScript's field leaves.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")
const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")

## The builders' and the waves' events (DomainProduction, FormationProduction).
const PRODUCED := ["from_reserve", "built", "to_reserve", "wave_full", "departed"]
const WAVES := ["A", "B"]


## [captained, seed, rolls, ticks, sends ([tick, wave (0: A, 1: B)] each)]: a full wave on
## B; partial waves, closed up and centred, then A again; both at once, to a rout.
func _runs() -> Array:
	return [
		[false, 1, true, 400, [[120, 1]]],
		[true, 2, false, 500, [[30, 0], [50, 1], [200, 0]]],
		[false, 3, true, 900, [[90, 0], [90, 1]]],
	]


## GDScript's field played: the crate's state and the builders' and waves' events.
func _by_gdscript(run: Array) -> Array:
	var field := FormationFieldActions.field(run[1], run[0])
	field.sim.blow_rolls = run[2]
	var produced := 0
	for tick in range(run[3]):
		for send in run[4]:
			if send[0] == tick:
				field.send(WAVES[send[1]])
		var events := field.step()
		produced += events.filter(func(e): return e["type"] in PRODUCED).size()
	for squad in field.sim.squads():
		squad.living()
	return [field.sim._field.snapshot(0, 0), produced]


## The crate's field played: its state and what `field_play` returns.
func _by_crate(run: Array) -> Array:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var sends := PackedInt64Array()
	for send in run[4]:
		sends.append_array(send)
	var at := PackedInt64Array([run[1], int(run[2]), int(NativeKernels.threaded), run[3]])
	var played: PackedInt64Array = sim._field.field_play(
		CrateContent.store(), PackedInt64Array([int(run[0])]), at, sends
	)
	return [sim._field.snapshot(0, 0), played]


func test_the_crate_plays_the_field_s_waves_to_the_state_gdscript_does() -> void:
	for run in _runs():
		var by_gdscript := _by_gdscript(run)
		var by_crate := _by_crate(run)
		var label := "captained %s s%d sends %s" % [run[0], run[1], run[4]]
		var played := Array(by_crate[1])
		assert_array(played).override_failure_message(label).is_equal([run[3], by_gdscript[1]])
		var differs := "the crate's field differs from GDScript's: %s" % label
		var same: bool = by_crate[0] == by_gdscript[0]
		assert_bool(same).override_failure_message(differs).is_true()


func test_the_comparison_tells_one_send_from_another() -> void:
	var by_gdscript := _by_gdscript([false, 1, true, 200, [[120, 1]]])
	var by_crate := _by_crate([false, 1, true, 200, [[121, 1]]])
	assert_bool(by_crate[0] == by_gdscript[0]).is_false()


func test_nothing_is_played_without_its_numbers() -> void:
	var sim := FormationSimulation.new(2.0, BattleTrials.TICK)
	var played: PackedInt64Array = sim._field.field_play(
		CrateContent.store(), PackedInt64Array(), PackedInt64Array([1, 1]), PackedInt64Array()
	)
	assert_array(Array(played)).is_equal([-1])
