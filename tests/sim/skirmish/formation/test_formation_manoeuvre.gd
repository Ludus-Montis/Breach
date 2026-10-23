extends GdUnitTestSuite
## Manoeuvres, per Decision 94 and spec 27 round 7: a formation always has one, the
## highest-priority that applies - combat, its route, re-forming, then the player's order -
## and marches only on the order. A re-formed line facing an enemy that has stopped closing
## in is let go, so the formation re-forms and marches on; contact mid-re-form is combat.

const FormationTaking = preload("res://sim/skirmish/formation/formation_taking.gd")
const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const FormationManoeuvre = preload("res://sim/skirmish/formation/formation_manoeuvre.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1


func _def(discipline: int = 30) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 400
	unit_def.items = [WeaponDef.innate_weapon(1)]
	unit_def.speed = 1.0
	unit_def.discipline = discipline
	return unit_def


func _row(columns: int, discipline: int = 30) -> Array:
	var placements := []
	for column in range(columns):
		placements.append([_def(discipline), Vector2i(0, column)])
	return placements


func _run(sim: FormationSimulation, ticks: int) -> Array:
	var log := []
	for _i in range(ticks):
		log.append_array(sim.step())
	return log


func _sim() -> FormationSimulation:
	var sim := FormationSimulation.new(2.0, TICK)
	return sim


## A player squad marching east along y = 20, re-formed to face south.
func _faced_south(sim: FormationSimulation) -> SkirmishSquad:
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 20), Vector2(128, 20)]))
	var squad := sim.spawn_squad(4, _row(4), "player", true, 0, east)
	squad.front_distance = 30.0 / 64.0
	FormationMarch.sync_units([squad])  # placed there
	sim.step()
	squad.stance = {"anchor": squad.position, "heading": 180.0}
	return squad


func test_a_marching_formation_is_on_its_order() -> void:
	var sim := _sim()
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 20), Vector2(128, 20)]))
	var squad := sim.spawn_squad(4, _row(4), "player", true, 0, east)

	_run(sim, 5)

	assert_int(squad.manoeuvre).is_equal(FormationManoeuvre.Kind.ORDER)


func test_facing_an_enemy_that_is_not_closing_in_it_re_forms_and_marches_on() -> void:
	var sim := _sim()
	var squad := _faced_south(sim)
	var hold := FormationRoute.new(PackedVector2Array([Vector2(30, 28), Vector2(30, 64)]))
	var still := sim.spawn_squad(4, _row(4), "the_kingdom", true, 0, hold)
	sim.order(still.id, SkirmishUnit.Order.HOLD)
	var start := squad.position.x

	sim.step()
	assert_int(squad.manoeuvre).is_equal(FormationManoeuvre.Kind.RE_FORM)
	_run(sim, 100)

	assert_bool(squad.stance.is_empty()).is_true()
	assert_int(squad.manoeuvre).is_equal(FormationManoeuvre.Kind.ORDER)
	assert_float(squad.position.x).is_greater(start + 2.0)


func test_a_formation_ordered_to_hold_keeps_facing_an_enemy_near() -> void:
	var sim := _sim()
	var squad := _faced_south(sim)
	sim.order(squad.id, SkirmishUnit.Order.HOLD)
	var hold := FormationRoute.new(PackedVector2Array([Vector2(30, 26), Vector2(30, 64)]))
	var still := sim.spawn_squad(4, _row(4, 60), "the_kingdom", true, 0, hold)
	sim.order(still.id, SkirmishUnit.Order.HOLD)
	squad.units[0].discipline = 200  # disciplined enough to hold a re-formed line

	_run(sim, 30)

	assert_bool(squad.stance.is_empty()).is_false()


func test_contact_mid_re_form_is_combat() -> void:
	var sim := _sim()
	var squad := _faced_south(sim)
	var near := FormationRoute.new(PackedVector2Array([Vector2(29.5, 23), Vector2(29.5, 64)]))
	var foe := sim.spawn_squad(2, _row(2), "the_kingdom", true, 0, near)
	sim.order(foe.id, SkirmishUnit.Order.HOLD)

	_run(sim, 3)

	assert_int(squad.manoeuvre).is_equal(FormationManoeuvre.Kind.COMBAT)
	assert_bool(foe.is_destroyed()).is_false()


func test_a_wave_re_forming_at_its_corner_as_a_fights_does_not_stick() -> void:
	# the round 6 feel test: B re-formed facing the line fighting A, and stayed there
	var field := FormationField.new(
		TICK,
		CrateContent.unit("grem"),
		4,
		CrateContent.unit("kingdom_militia"),
		CrateContent.unit("grem_chieftain"),
		null,
		727784
	)
	field.waves["A"].route = field.routes["C"]
	for _i in range(3000):
		if field.waves["A"].built() == 8 and field.waves["B"].built() == 9:
			break
		field.step()
	var wave := field.send("B")
	for _i in range(10):
		field.step()
	field.send("A")

	var still := 0
	var longest := 0
	var last := wave.position
	for _i in range(1200):
		field.step()
		var moving := wave.state in [SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING]
		moving = moving and not FormationTaking.holding(wave)  # a hold to take the downed
		still = still + 1 if moving and wave.position == last else 0
		longest = maxi(longest, still)
		last = wave.position

	# Not stuck: its longest pause is a regroup for a unit come to and walking back to join
	# it (Decision 126), now a little later after it holds to take its downed foes.
	assert_int(longest).is_less(90)
