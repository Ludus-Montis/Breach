extends GdUnitTestSuite
## The whole tick as one crate call (FormationTick.step, `tick_run`, spec 39 step 8b5-xlii):
## a clash fought to its end (the fight, the surrender, the downed, carrying, the rout) leaves
## the events and GDScript's copies tick by tick as the opening, the scrum, the aftermath,
## the mustering and the end called one after another did, and no mirror run after the call
## writes over what a later pass in it did: each unit's state is the crate's. The crate builds
## each pass's rules from its own battle tuning as each pass's `args` builds them (8c-i).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationBench = preload("res://sim/skirmish/formation/formation_bench.gd")
const FormationScrum = preload("res://sim/skirmish/formation/formation_scrum.gd")
const FormationAftermath = preload("res://sim/skirmish/formation/formation_aftermath.gd")
const FormationMustering = preload("res://sim/skirmish/formation/formation_mustering.gd")
const FormationEnding = preload("res://sim/skirmish/formation/formation_ending.gd")
const FormationOpening = preload("res://sim/skirmish/formation/formation_opening.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const TICKS := 600


## Twelve grems against twelve faint-hearted militia, four wide, blows rolled; each side
## carries its downed. The militia break and flee home; some surrender, some come to.
func _clash() -> FormationSimulation:
	var sim := FormationBench.clash(12, 4, 3)
	sim.blow_rolls = true
	for squad: SkirmishSquad in sim.squads():
		squad.tends = "carry"
		if squad.faction_id != "player":
			for unit: SkirmishUnit in squad.units:
				unit.courage = 10
	return sim


## A player's squad of three holding at home in one file, carrying its downed; its middle
## unit felled (0 hp, on the fell queue), so the deaths down it and carrying bears it in the
## same tick. Returns the sim and that unit's id.
func _felled() -> Array:
	var sim := FormationSimulation.new(2.0, 0.1)
	sim.fight_seed = 3
	var grem: Resource = CrateContent.unit("grem")
	var placements := [[grem, Vector2i(0, 0)], [grem, Vector2i(1, 0)], [grem, Vector2i(2, 0)]]
	var squad := sim.spawn_squad(1, placements, "player", true)
	squad.tends = "carry"
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	sim.step()
	var felled: SkirmishUnit = squad.units[1]
	UnitHealth.put_one(squad, felled, 0)
	return [sim, felled.id]


## One tick as FormationSimulation made it before, a call a part.
func _phased(sim: FormationSimulation) -> Array:
	sim._tick += 1
	var events := []
	sim._open(events)
	var squads := sim._squads
	var seconds := sim.tick_seconds
	var pace := FormationSimulation.TRAVEL_SCALE * MapLayoutDef.CELLS_PER_TILE
	var seed := sim.fight_seed
	events.append_array(
		FormationScrum.step(squads, sim._tick, pace, seconds, seed, sim.terrain, sim._field)
	)
	var per_second := sim._attack_interval_ticks()
	var ticks := [per_second, seed, sim._tick]
	var strays := FormationAftermath.step(squads, ticks, seconds, sim.blow_rolls, null, events)
	var at := [sim._tick, per_second, seed, sim._next_squad_id]
	sim._next_squad_id = FormationMustering.step(squads, strays, at, [pace, seconds], null, events)
	var end := [seconds, FormationSimulation.TRAVEL_SCALE, pace]
	events.append_array(FormationEnding.step(squads, [sim._tick, seed], end, null))
	sim.store.settle(squads)
	return events


## Each squad's id, state, morale and front, and each of its units' id, state (GDScript's
## copy and the crate's), hp, wounds and place.
func _state(sim: FormationSimulation) -> Array:
	var columns: UnitColumns = sim.store.columns
	var out := []
	for squad: SkirmishSquad in sim._squads:
		var units := []
		for unit: SkirmishUnit in squad.units:
			(
				units
				. append(
					[
						unit.id,
						unit.state,
						columns.value(unit.slot, UnitColumns.STATE),
						columns.value(unit.slot, UnitColumns.HP),
						unit.wounded,
						unit.carrying,
						unit.carried_by,
						columns.value(unit.slot, UnitColumns.POSITION),
					]
				)
			)
		out.append([squad.id, squad.state, squad.morale, squad.position, units])
	return out


## The kinds of event told, once each.
func _kinds(events: Array, kinds: Dictionary) -> void:
	for event in events:
		kinds[event["type"]] = true


func test_the_one_call_leaves_the_events_and_copies_as_the_passes_one_by_one() -> void:
	var phased := _clash()
	var whole := _clash()
	var kinds := {}
	for tick in range(TICKS):
		var told := [_phased(phased), whole.step()]
		_kinds(told[0], kinds)
		assert_array(told[1]).is_equal(told[0])
		assert_array(_state(whole)).is_equal(_state(phased))
		if is_failure():
			return
	for kind in ["routed", "surrendered", "downed", "borne", "came_to", "fled_home"]:
		assert_bool(kinds.has(kind)).override_failure_message(kind + " never told").is_true()


func test_each_unit_s_state_is_the_crate_s_after_the_one_call() -> void:
	var sim := _clash()
	var columns: UnitColumns = sim.store.columns
	for tick in range(TICKS):
		sim.step()
		for squad: SkirmishSquad in sim._squads:
			var states := squad.units.map(func(u): return u.state)
			var crate := squad.units.map(func(u): return columns.value(u.slot, UnitColumns.STATE))
			assert_array(states).is_equal(crate)
		if is_failure():
			return


func test_a_body_the_deaths_down_and_carrying_bears_in_one_call_stays_borne() -> void:
	var phased: Array = _felled()
	var whole: Array = _felled()

	var told := [_phased(phased[0]), whole[0].step()]

	assert_array(told[1]).is_equal(told[0])
	assert_array(told[0].map(func(e): return e["type"])).contains(["downed", "borne"])
	assert_array(_state(whole[0])).is_equal(_state(phased[0]))
	var body: SkirmishUnit = whole[0].store.record_of(whole[1])
	assert_int(body.state).is_equal(SkirmishUnit.State.CARRIED)
	var crate: int = whole[0].store.columns.value(body.slot, UnitColumns.STATE)
	assert_int(crate).is_equal(SkirmishUnit.State.CARRIED)


func test_the_crate_builds_each_pass_s_rules_as_its_args_build_them() -> void:
	var sim := FormationBench.clash(2, 2, 3)
	var field: Object = ColumnsLent.field(sim.store.columns)
	var travel := FormationSimulation.TRAVEL_SCALE
	var cells := travel * MapLayoutDef.CELLS_PER_TILE
	var paces := PackedFloat64Array([travel, 0.1, 12.0, cells])
	for tick in [30, 31]:  # a second's tick (morale steps) and not
		var at := PackedInt64Array([tick, 3, 10, 1, int(NativeKernels.threaded)])
		var given: Array = field.tick_given(CrateContent.store(), at, paces)
		var expected := [
			FormationOpening.args([tick, 3], [travel, 0.1, 12.0]),
			FormationScrum.args(tick, cells, 0.1, 3),
			FormationAftermath.args([10, 3, tick], 0.1, true),
			FormationMustering.args([tick, 10, 3], [cells, 0.1]),
			FormationEnding.args([tick, 3], [0.1, travel, cells]),
		]
		assert_array(given).is_equal(expected)
