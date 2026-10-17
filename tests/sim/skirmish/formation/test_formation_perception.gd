extends GdUnitTestSuite
## FormationPerception (Decision 134): each formation sees, once a tick, the standing hostile
## formations it detects, and is threatened if it sees one. A threatened group fights
## before it takes the downed: it sends no takers, and calls back any it has out.

const FormationPerception = preload("res://sim/skirmish/formation/formation_perception.gd")
const FormationTaking = preload("res://sim/skirmish/formation/formation_taking.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const GREM := "res://content/units/grem.tres"
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")


## A holding squad of `count` grems of `faction`, its front `front` cells along.
func _squad(sim: FormationSimulation, count: int, faction: String, front: float) -> SkirmishSquad:
	var grem: UnitDef = load(GREM)
	var placements := []
	for column in count:
		placements.append([grem, Vector2i(0, column)])
	var squad := sim.spawn_squad(count, placements, faction, faction == "player")
	squad.front_distance = front / 64.0
	squad.order = SkirmishUnit.Order.HOLD
	FormationMarch.sync_units([squad])
	return squad


func _gap(a: SkirmishSquad, b: SkirmishSquad) -> float:
	return UnitPlaces.of(a, a.units[0]).distance_to(UnitPlaces.of(a, b.units[0]))


func test_formations_within_sight_see_each_other_and_are_threatened() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var mine := _squad(sim, 3, "player", 40.0)
	var theirs := _squad(sim, 3, "the_kingdom", 60.0)
	assert_float(_gap(mine, theirs)).is_less(mine.units[0].detection)

	var seen := FormationPerception.see(sim.squads())

	assert_array(seen[mine.id]["seen"]).contains_exactly([theirs])
	assert_bool(FormationPerception.threatened(seen, mine)).is_true()
	assert_bool(FormationPerception.threatened(seen, theirs)).is_true()


func test_a_foe_beyond_sight_is_no_threat() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var mine := _squad(sim, 3, "player", 40.0)
	var theirs := _squad(sim, 3, "the_kingdom", 200.0)
	assert_float(_gap(mine, theirs)).is_greater(mine.units[0].detection)

	var seen := FormationPerception.see(sim.squads())

	assert_bool(FormationPerception.threatened(seen, mine)).is_false()
	assert_array(seen[mine.id]["seen"]).is_empty()


func test_friends_and_routers_are_no_threat() -> void:
	var sim := FormationSimulation.new(4.0, 0.1)
	var mine := _squad(sim, 3, "player", 40.0)
	_squad(sim, 3, "player", 50.0)
	var fleeing := _squad(sim, 3, "the_kingdom", 60.0)
	fleeing.state = SkirmishSquad.State.ROUTING

	var seen := FormationPerception.see(sim.squads())

	assert_bool(FormationPerception.threatened(seen, mine)).is_false()


## A holding player line with a downed kingdom unit 4 cells ahead, and - if `foe_at` is
## given - a standing kingdom squad that far along. Returns [sim, line].
func _downed_ahead(foe_at: float) -> Array:
	var sim := FormationSimulation.new(4.0, 0.1)
	var line := _squad(sim, 3, "player", 40.0)
	var fallen := _squad(sim, 1, "the_kingdom", 44.0)
	var body: SkirmishUnit = fallen.units[0]
	UnitHealth.put_one(fallen, body, 0)
	UnitState.put(fallen, body, SkirmishUnit.State.DOWNED)
	fallen.store.columns.put(body.slot, UnitColumns.WAKE_LEFT, 999.0)
	if foe_at > 0.0:
		_squad(sim, 3, "the_kingdom", foe_at)
	return [sim, line]


func _sent(sim: FormationSimulation, ticks: int) -> bool:
	var log := []
	for _i in ticks:
		log.append_array(sim.step())
	return log.any(func(e): return e["type"] == "sent_to_take")


func test_an_unthreatened_group_takes_the_downed() -> void:
	assert_bool(_sent(_downed_ahead(0.0)[0], 1)).is_true()


func test_a_threatened_group_sends_no_takers() -> void:
	var setup := _downed_ahead(70.0)  # a standing foe in sight, out of reach
	assert_bool(_sent(setup[0], 1)).is_false()
	assert_bool(FormationTaking.holding(setup[1])).is_false()


func test_takers_out_come_back_when_a_foe_appears() -> void:
	var setup := _downed_ahead(0.0)
	var sim: FormationSimulation = setup[0]
	assert_bool(_sent(sim, 1)).is_true()
	assert_bool(FormationTaking.holding(setup[1])).is_true()
	var foe := _squad(sim, 3, "the_kingdom", 70.0)
	assert_int(foe.units.size()).is_equal(3)

	sim.step()

	assert_bool(FormationTaking.holding(setup[1])).is_false()
