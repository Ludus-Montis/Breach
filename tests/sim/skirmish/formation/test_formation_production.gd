extends GdUnitTestSuite
## FormationProduction, per Decisions 42 and 45: one lane's wave - its painted template
## and which places are filled. The domain's builders fill it (fill); a new template
## takes effect at once, filled units fold in, and the leftovers are handed back for the
## domain reserve. A full wave is announced once and departs manually or automatically.

const FormationProduction = preload("res://sim/skirmish/formation/formation_production.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const WaveTemplate = preload("res://sim/skirmish/formation/wave_template.gd")
const WavePresets = preload("res://sim/skirmish/formation/wave_presets.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

const TICK := 0.1

var _grem: UnitDef
var _brute: UnitDef


func before_test() -> void:
	_grem = _def(20)
	_brute = _def(60, 2, 2)


func _def(hp: int, depth: int = 1, width: int = 1) -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = hp
	unit_def.items = [WeaponDef.innate_weapon(5)]
	unit_def.speed = 1.0
	unit_def.footprint_depth = depth
	unit_def.footprint_width = width
	return unit_def


func _production(template: WaveTemplate) -> FormationProduction:
	var production := FormationProduction.new("player", true)
	production.set_template(template)
	return production


func _fill(production: FormationProduction, unit_def: UnitDef, count: int) -> void:
	for i in range(count):
		production.fill(unit_def)


func _run(production: FormationProduction, sim: FormationSimulation, ticks: int) -> Array:
	var events := []
	for i in range(ticks):
		events.append_array(production.step(sim))
	return events


func _count(events: Array, kind: String) -> int:
	return events.filter(func(e): return e["type"] == kind).size()


func test_units_fill_their_places_front_first() -> void:
	var production := _production(WavePresets.line(_grem, 5, 5))

	_fill(production, _grem, 2)

	assert_int(production.built()).is_equal(2)
	var filled := production.preview().filter(func(p): return p[4])
	(
		assert_array(filled.map(func(p): return Vector2i(p[0], p[1])))
		. is_equal([Vector2i(0, 0), Vector2i(0, 1)])
	)


func test_wanted_counts_the_unfilled_places_by_type() -> void:
	var template := WaveTemplate.new(5, 8)
	template.paint(_brute, Vector2i(0, 1))
	template.paint(_grem, Vector2i(2, 1))
	template.paint(_grem, Vector2i(2, 2))
	var production := _production(template)

	production.fill(_grem)

	assert_dict(production.wanted()).is_equal({_grem: 1, _brute: 1})
	assert_bool(production.fill(_brute)).is_true()
	assert_bool(production.fill(_brute)).is_false()  # no brute place left


func test_a_new_template_takes_effect_at_once_and_folds_filled_units_in() -> void:
	var production := _production(WavePresets.line(_grem, 5, 5))
	_fill(production, _grem, 4)

	var events := production.set_template(WavePresets.line(_grem, 3, 3))

	assert_int(production.built()).is_equal(3)
	assert_dict(production.wanted()).is_empty()
	assert_int(events[0]["banked"]).is_equal(1)
	assert_array(events[0]["leftovers"]).is_equal([_grem])


func test_units_of_a_type_the_new_shape_lacks_are_all_handed_back() -> void:
	var production := _production(WavePresets.line(_grem, 4, 4))
	_fill(production, _grem, 2)
	var brute_only := WaveTemplate.new(4, 8)
	brute_only.paint(_brute, Vector2i(0, 1))

	var events := production.set_template(brute_only)

	assert_int(production.built()).is_equal(0)
	assert_int(events[0]["leftovers"].size()).is_equal(2)


func test_a_full_wave_is_announced_once() -> void:
	var production := _production(WavePresets.line(_grem, 3, 3))
	var sim := FormationSimulation.new(9.0, TICK)
	_fill(production, _grem, 3)

	var events := _run(production, sim, 10)

	assert_int(_count(events, "wave_full")).is_equal(1)
	assert_array(sim.squads()).is_empty()


func test_send_deploys_the_painted_layout_and_restarts_the_same_template() -> void:
	var template := WaveTemplate.new(5, 8)
	template.paint(_brute, Vector2i(0, 1))
	template.paint(_grem, Vector2i(2, 1))
	template.paint(_grem, Vector2i(2, 2))
	var production := _production(template)
	var sim := FormationSimulation.new(9.0, TICK)
	production.fill(_brute)
	_fill(production, _grem, 2)

	var squad = production.send(sim)

	assert_int(squad.width).is_equal(2)  # painted columns 1..2
	var brute_unit = squad.units.filter(func(u): return u.footprint_width == 2)[0]
	assert_int(UnitPlace.rank(squad, brute_unit)).is_equal(0)  # the brute leads
	assert_int(production.built()).is_equal(0)
	assert_int(production.preview().size()).is_equal(3)


func test_auto_departure_sends_the_wave_when_it_fills() -> void:
	var production := _production(WavePresets.line(_grem, 3, 3))
	production.departure = FormationProduction.Departure.AUTO_WHEN_FULL
	var sim := FormationSimulation.new(9.0, TICK)
	_fill(production, _grem, 3)

	var events := _run(production, sim, 1)

	assert_int(_count(events, "departed")).is_equal(1)
	assert_int(sim.squads().size()).is_equal(1)
	assert_int(sim.squads()[0].units.size()).is_equal(3)


func test_an_empty_template_wants_nothing_and_sends_nothing() -> void:
	var production := _production(WaveTemplate.new(3, 0))
	var sim := FormationSimulation.new(9.0, TICK)

	assert_dict(production.wanted()).is_empty()
	assert_bool(production.fill(_grem)).is_false()
	assert_object(production.send(sim)).is_null()


func test_a_partial_wave_closes_up_with_its_leader_centred_behind() -> void:
	var leader := _def(40)
	leader.preferred_position = UnitDef.Position.MID
	var template := WaveTemplate.new(8, 9)
	for column in range(8):
		template.paint(_grem, Vector2i(0, column))
	template.paint(leader, Vector2i(1, 6))  # painted off to one side
	var production := _production(template)
	_fill(production, _grem, 4)
	production.fill(leader)
	var sim := FormationSimulation.new(2.0, TICK)

	var squad := production.send(sim)

	assert_int(squad.width).is_equal(4)
	var front: Array = squad.units.filter(func(u): return UnitPlace.rank(squad, u) == 0)
	assert_int(front.size()).is_equal(4)
	var behind: Array = squad.units.filter(func(u): return UnitPlace.rank(squad, u) == 1)
	assert_int(behind.size()).is_equal(1)
	assert_int(UnitPlace.of(squad, behind[0]).y).is_between(1, 2)  # centred behind the four
