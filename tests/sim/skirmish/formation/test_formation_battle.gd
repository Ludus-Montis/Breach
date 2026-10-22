extends GdUnitTestSuite
## FormationBattle, per Decision 45: the lanes, both domains and the slot pool of the
## formation feel test, so the scene is only glue. The player's builders fill the lanes'
## waves by the chosen rule; reshapes bank into the domain reserve; the kingdom builds
## its militia lines only when asked.

const FormationBattle = preload("res://sim/skirmish/formation/formation_battle.gd")
const DomainProduction = preload("res://sim/skirmish/formation/domain_production.gd")
const WavePresets = preload("res://sim/skirmish/formation/wave_presets.gd")

const GREM := "grem"
const BRUTE := "grem_brute"
const MILITIA := "kingdom_militia"


func _battle() -> FormationBattle:
	var battle := FormationBattle.new(8, {"c": 5, "k": 4}, 0.1, CrateContent.unit(MILITIA), 3)
	battle.add_lane("c", 9.0, WavePresets.line(CrateContent.unit(GREM), 2, 5))
	battle.add_lane("k", 9.0, WavePresets.line(CrateContent.unit(GREM), 2, 4))
	battle.player.set_builders(CrateContent.unit(GREM), 1)
	return battle


func _run(battle: FormationBattle, ticks: int, with_kingdom: bool = false) -> Dictionary:
	var events := {}
	for i in range(ticks):
		var tick_events := battle.step(with_kingdom)
		for key in tick_events:
			events[key] = events.get(key, []) + tick_events[key]
	return events


func test_the_pool_starts_with_what_each_lane_has_painted() -> void:
	var battle := _battle()

	assert_int(battle.pool.assigned("c")).is_equal(2)
	assert_int(battle.pool.assigned("k")).is_equal(2)
	assert_int(battle.allowance("c")).is_equal(6)  # its own 2 plus the 4 free


func test_one_builder_shares_grems_out_in_turn() -> void:
	var battle := _battle()
	battle.player.distribution = DomainProduction.Distribution.ROUND_ROBIN

	_run(battle, 40)  # a grem every 2 s

	assert_int(battle.lane("c").production.built()).is_equal(1)
	assert_int(battle.lane("k").production.built()).is_equal(1)


func test_reshaping_banks_the_displaced_units_in_the_domain_reserve() -> void:
	var battle := _battle()
	battle.player.set_builders(CrateContent.unit(GREM), 2)
	_run(battle, 40)  # both of lane c's grems
	battle.lane("c").brush = CrateContent.unit(BRUTE)

	var banked := battle.paint("c", Vector2i(0, 0))

	assert_int(banked).is_equal(2)
	assert_int(battle.player.reserve.size()).is_equal(2)
	assert_int(battle.pool.assigned("c")).is_equal(4)


func test_erasing_in_one_lane_frees_slots_for_the_other() -> void:
	var battle := _battle()

	battle.paint("k", Vector2i(0, 0), true)

	assert_int(battle.pool.assigned("k")).is_equal(1)
	assert_int(battle.allowance("c")).is_equal(7)


func test_the_kingdom_builds_its_lines_only_when_asked() -> void:
	var battle := _battle()
	_run(battle, 200)
	var idle := battle.lane("c").sim.squads().any(func(s): return s.faction_id == "the_kingdom")

	_run(battle, 200, true)  # two militia builders, 5 s each, three to a line

	assert_bool(idle).is_false()
	(
		assert_bool(
			battle.lane("c").sim.squads().any(func(s): return s.faction_id == "the_kingdom")
		)
		. is_true()
	)
