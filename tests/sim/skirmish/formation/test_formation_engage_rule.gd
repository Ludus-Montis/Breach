extends GdUnitTestSuite
## Who may be engaged and who seeks combat, per Decision 111: any standing squad may be
## engaged - what a squad attacks is the attacker's choice - but only one not getting away
## (retreating or routing) seeks combat, and one getting away doesn't turn to fight back
## as a squad. Who may be engaged and who seeks it, and that a rout is met where its units
## are, are the crate's scenarios (`contact/tests.rs`, spec 39 step 4c).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")


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
