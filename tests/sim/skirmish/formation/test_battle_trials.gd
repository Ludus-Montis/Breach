extends GdUnitTestSuite
## Battle seeds and trials, per Decision 93: each blow is rolled (BlowLanding), seeded by
## the battle; the same seed replays a battle exactly, a new one varies it; and
## the trials runner fights a scenario once per seed and summarises the spread.

const BattleTrials = preload("res://sim/skirmish/formation/battle_trials.gd")


func test_the_same_seed_replays_a_battle() -> void:
	var first := BattleTrials.run("mirror_headon", 1, {"first_seed": 5})
	var again := BattleTrials.run("mirror_headon", 1, {"first_seed": 5})

	assert_array(again).is_equal(first)


func test_new_seeds_vary_a_battle() -> void:
	var rolled := BattleTrials.run("mirror_headon", 6)

	var outcomes := {}
	for result in rolled:
		outcomes[[result["lost"], result["ticks"]]] = true
	assert_int(outcomes.size()).is_greater(1)


func test_a_mirror_fight_is_even_and_a_mirror_flank_is_not() -> void:
	var headon := BattleTrials.summary(BattleTrials.run("mirror_headon", 40))  # 12 swing by chance
	var flank := BattleTrials.summary(BattleTrials.run("mirror_flank", 6))

	var lost: Dictionary = headon["lost"]
	assert_float(absf(lost["player"][0] - lost["the_kingdom"][0])).is_less(1.5)
	assert_int(flank["wins"]["player"]).is_equal(6)
