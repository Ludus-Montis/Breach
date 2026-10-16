extends GdUnitTestSuite
## Who may be engaged and who seeks combat, per Decision 111: any standing squad may be
## engaged - what a squad attacks is the attacker's choice - but only one not getting away
## (retreating or routing) seeks combat, and one getting away doesn't turn to fight back
## as a squad.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _squad(state: SkirmishSquad.State, order: SkirmishUnit.Order) -> SkirmishSquad:
	var unit := SkirmishUnit.new()
	unit.id = 1
	_places.health(unit, 10)
	var squad := SkirmishSquad.new(
		1, "player", 1, 0.0, 1, [unit] as Array[SkirmishUnit], null, _places.store
	)
	squad.state = state
	squad.order = order
	return squad


func test_any_standing_squad_may_be_engaged_but_only_one_not_getting_away_seeks_combat() -> void:
	var holding := _squad(SkirmishSquad.State.HOLDING, SkirmishUnit.Order.HOLD)
	var retreating := _squad(SkirmishSquad.State.MOVING, SkirmishUnit.Order.RETREAT)
	var routing := _squad(SkirmishSquad.State.ROUTING, SkirmishUnit.Order.HOLD)
	var destroyed := _squad(SkirmishSquad.State.DESTROYED, SkirmishUnit.Order.HOLD)

	for squad in [holding, retreating, routing]:
		assert_bool(FormationContact.engageable(squad)).is_true()
	assert_bool(FormationContact.engageable(destroyed)).is_false()
	assert_bool(FormationContact.can_engage(holding)).is_true()
	assert_bool(FormationContact.can_engage(retreating)).is_false()
	assert_bool(FormationContact.can_engage(routing)).is_false()


func test_a_retreating_squad_in_reach_is_engaged_but_does_not_turn_to_fight() -> void:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	var sim := FormationSimulation.new(9.0, 0.1)
	var mine := sim.spawn_squad(1, [[unit_def, Vector2i(0, 0)]], "player", true)
	var theirs := sim.spawn_squad(1, [[unit_def, Vector2i(0, 0)]], "the_kingdom", false)
	theirs.pursues = false  # its frame holds its post; its units still fight in reach
	for _i in range(600):
		if sim.step().any(func(e): return e["type"] == "engaged"):
			break
	sim.order(mine.id, SkirmishUnit.Order.RETREAT)
	var relocked := false
	for _i in range(5):
		sim.step()
		relocked = relocked or theirs.engaged_with == mine.id

	assert_bool(relocked).is_true()  # it may be engaged as it goes
	assert_int(mine.engaged_with).is_equal(0)  # it doesn't turn to fight
	assert_int(mine.order).is_equal(SkirmishUnit.Order.RETREAT)


func test_a_routing_squad_is_met_where_its_units_are_not_at_its_frame() -> void:
	# A feel-test field run: a line's last router fled to its reserve while its frame stood
	# at the wave's front; the wave locked onto the empty frame tick after tick.
	var routing := _squad(SkirmishSquad.State.ROUTING, SkirmishUnit.Order.HOLD)
	var holding := _squad(SkirmishSquad.State.HOLDING, SkirmishUnit.Order.HOLD)

	assert_bool(FormationContact.framed(routing)).is_false()
	assert_bool(FormationContact.framed(holding)).is_true()
	var front := _squad(SkirmishSquad.State.MOVING, SkirmishUnit.Order.ADVANCE)
	routing.faction_id = "the_kingdom"
	routing.heading = 270.0
	routing.position = front.position + Vector2(0.5, 0.0)  # front to front, in melee reach
	assert_object(FormationContact.nearest_hostile(front, [front, routing])).is_null()
