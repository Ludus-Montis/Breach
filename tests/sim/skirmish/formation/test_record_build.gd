extends GdUnitTestSuite
## A record of play names the build that played it (Decision 130, SimBuild): the header
## carries the build, a record read back gives it, an older record without one still
## reads, and a replay on another build - or of an unstamped record - is warned about.

const FormationRecord = preload("res://sim/skirmish/formation/formation_record.gd")


func test_the_build_is_a_stable_fingerprint_of_the_simulation() -> void:
	var build := SimBuild.id()
	assert_int(build.length()).is_equal(12)
	assert_bool(build.is_valid_hex_number()).is_true()
	assert_str(SimBuild.id()).is_equal(build)


func test_a_record_carries_its_build_and_reads_it_back() -> void:
	var head := FormationRecord.header(7, true, "0123456789ab")
	assert_str(head).is_equal("record 2 seed 7 captain on build 0123456789ab")

	var read := FormationRecord.parse(head + "\n3 player send A")
	assert_str(read["build"]).is_equal("0123456789ab")
	assert_int(read["seed"]).is_equal(7)
	assert_bool(read["captained"]).is_true()
	assert_int(read["commands"].size()).is_equal(1)


func test_an_older_record_without_a_build_still_reads() -> void:
	var read := FormationRecord.parse("record 1 seed 9 captain off\n12 kingdom pursue all off")
	assert_str(read["build"]).is_equal("")
	assert_int(read["seed"]).is_equal(9)
	assert_int(read["commands"].size()).is_equal(1)


func test_a_replay_is_warned_about_only_when_its_build_is_not_this_one() -> void:
	assert_str(SimBuild.compare(SimBuild.id())).is_equal("")
	assert_str(SimBuild.compare("000000000000")).contains("may play out differently")
	assert_str(SimBuild.compare("")).contains("no build stamp")
