extends GdUnitTestSuite
## The feel test's orders (Decision 93) as commands of the record of play (Decision 115):
## given to a field as its buttons would, recorded with their ticks, and a record - or an
## old log of the feel test's words - replays the same battle tick for tick.

const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationRecord = preload("res://sim/skirmish/formation/formation_record.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")


func _state(field) -> String:
	var rows := []
	for squad in field.sim.squads():
		var units: Array = squad.units.map(
			func(u): return [u.id, UnitHealth.of_one(squad, u), UnitPlaces.of(squad, u)]
		)
		rows.append([squad.id, squad.state, squad.front_distance, units])
	return str(rows)


func _words(words: String) -> Dictionary:
	return FormationRecord.from_words(0, words)


func test_orders_apply_as_the_buttons_do() -> void:
	var field := FormationFieldActions.field(7, false)

	assert_bool(FormationFieldActions.apply(field, _words("wait on"))).is_true()
	assert_bool(field.waves["B"].staging.is_empty()).is_false()
	assert_bool(FormationFieldActions.apply(field, _words("via_c on"))).is_true()
	assert_object(field.waves["A"].route).is_same(field.routes["C"])
	assert_bool(FormationFieldActions.apply(field, _words("pursues on"))).is_true()
	assert_bool(field.kingdom_line.pursues).is_true()
	assert_bool(FormationFieldActions.apply(field, _words("dance"))).is_false()


func test_a_hurried_wave_runs_ahead_of_a_marching_one() -> void:
	var marching := FormationFieldActions.field(7, false)
	var hurried := FormationFieldActions.field(7, false)
	assert_bool(FormationFieldActions.apply(hurried, _words("hurry A on"))).is_true()
	var record := FormationRecord.line(_words("hurry A on"))
	for field in [marching, hurried]:
		for _i in range(300):  # A's wave built
			field.step()
		field.send("A")
		for _i in range(30):
			field.step()

	var ahead := func(field): return field.sim.squads()[-1].front_distance
	assert_float(ahead.call(hurried)).is_greater(ahead.call(marching))
	assert_bool(hurried.sim.squads()[-1].hurry).is_true()
	assert_str(record).is_equal("0 player hurry A on")
	FormationFieldActions.apply(hurried, _words("hurry A off"))
	hurried.step()
	assert_bool(hurried.sim.squads()[-1].hurry).is_false()


func test_a_waves_wounded_are_left_recovered_or_carried_as_ordered() -> void:
	var field := FormationFieldActions.field(7, false)

	assert_bool(FormationFieldActions.apply(field, _words("tend A carry"))).is_true()
	assert_str(field.tending["A"]).is_equal("carry")
	assert_bool(FormationFieldActions.apply(field, _words("tend A leave"))).is_true()
	assert_str(field.tending["A"]).is_equal("")
	assert_bool(FormationFieldActions.apply(field, _words("tend A eat"))).is_false()
	for _i in range(300):
		field.step()
	FormationFieldActions.apply(field, _words("tend A recover"))
	field.send("A")
	field.step()
	assert_str(field.sim.squads()[-1].tends).is_equal("recover")


func test_an_order_is_taken_only_from_the_side_that_gives_it() -> void:
	var field := FormationFieldActions.field(7, false)
	var theirs := FormationRecord.command(0, FormationRecord.KINGDOM, "send", "A")
	var unknown := FormationRecord.command(0, FormationRecord.PLAYER, "send", "Z")

	assert_bool(FormationFieldActions.apply(field, theirs)).is_false()
	assert_bool(FormationFieldActions.apply(field, unknown)).is_false()
	assert_bool(field.sim.squads().any(func(q): return q.faction_id == "player")).is_false()


func test_a_record_replays_the_same_battle() -> void:
	var field := FormationFieldActions.field(446157, false)
	var log := [FormationRecord.header(446157, false)]
	for _i in range(60):
		field.step()
	var send := FormationRecord.from_words(field.sim.tick_number(), "send A+B")
	FormationFieldActions.apply(field, send)
	log.append(FormationRecord.line(send))
	for _i in range(300):
		field.step()

	var replayed := FormationFieldActions.replay("\n".join(log), 300)

	assert_str(log[1]).is_equal("60 player send A+B")
	assert_int(replayed.sim.tick_number()).is_equal(field.sim.tick_number())
	assert_str(_state(replayed)).is_equal(_state(field))


func test_a_record_reads_back_as_its_version_seed_and_commands() -> void:
	var record := "record 1 seed 9 captain on\n0 player wait waves on\n12 kingdom pursue all off\n"
	var read := FormationFieldActions.parse(record)

	assert_int(read["version"]).is_equal(1)
	assert_int(read["seed"]).is_equal(9)
	assert_bool(read["captained"]).is_true()
	(
		assert_array(read["commands"].map(func(c): return FormationRecord.line(c)))
		. is_equal(["0 player wait waves on", "12 kingdom pursue all off"])
	)
	assert_bool(FormationFieldActions.parse("not a log").is_empty()).is_true()


func test_an_old_log_of_the_feel_tests_words_reads_as_commands() -> void:
	var read := FormationFieldActions.parse(
		"seed 9 captain on\n0 wait on\n12 via_c on\n30 send A+B"
	)

	assert_int(read["version"]).is_equal(0)
	(
		assert_array(read["commands"].map(func(c): return FormationRecord.line(c)))
		. is_equal(["0 player wait waves on", "12 player route A C", "30 player send A+B"])
	)


func test_waves_that_merge_after_a_fight_regroup_and_march_on() -> void:
	# A feel-test log: A (via C) and B beat the line at the crossroads, their places
	# overlapping; they used to stand there regrouping forever, knocking each other loose.
	var log := "seed 492625 captain off\n96 via_c on\n158 send A+B"

	var field := FormationFieldActions.replay(log, 1200)  # units walk to their places

	for squad in field.sim.squads():
		if squad.faction_id == "player" and squad.state != SkirmishSquad.State.DESTROYED:
			if squad.state == SkirmishSquad.State.ROUTING and squad.units.size() == 1:
				continue  # one come to on its own, making for home (Decision 126)
			assert_bool(squad.loose.is_empty()).is_true()
			assert_int(squad.state).is_equal(SkirmishSquad.State.ARRIVED)


func test_waves_short_of_a_slot_wait_by_the_fight_not_in_their_places() -> void:
	# A feel-test log: as the line shrank, A's units with no slot left walked back into
	# their places while the rest fought on.
	var log := "seed 492625 captain off\n22 via_c on\n129 send A+B"
	var walked_back := {"units": 0}  # a lambda captures a local int by value
	var on_events := func(field, _events):
		if field.kingdom_line.is_destroyed():
			return
		for squad in field.sim.squads():
			if squad.faction_id != "player" or squad.state != SkirmishSquad.State.FIGHTING:
				continue
			for unit in squad.living():
				var columns: UnitColumns = squad.store.columns
				var idle: bool = (
					not squad.loose.has(unit.id)
					or (
						EntryKeys.goal(columns, unit.slot) == null
						and columns.value(unit.slot, UnitColumns.TOUCH) == 0
					)
				)
				if unit.preferred_position == 0 and idle:
					walked_back["units"] += 1

	FormationFieldActions.replay(log, 280, on_events)

	assert_int(walked_back["units"]).is_equal(0)
