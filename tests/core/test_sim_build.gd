extends GdUnitTestSuite
## Which build of the simulation is running (Decision 130, SimBuild): a stable fingerprint of
## everything that decides a battle - since spec 39 step 9d-iv, the crate's source and the
## content - that a record of play carries, and a replay on another build is warned about.
## The record's header itself is the crate's (`record.rs`).


func test_the_build_is_a_stable_fingerprint_of_the_simulation() -> void:
	var build := SimBuild.id()
	assert_int(build.length()).is_equal(12)
	assert_bool(build.is_valid_hex_number()).is_true()
	assert_str(SimBuild.id()).is_equal(build)


func test_the_build_reads_the_crates_rules_and_the_content() -> void:
	var sources := SimBuild.sources()

	assert_array(sources).contains(["res://native/rust/src/lib.rs"])
	assert_array(sources).contains(["res://content/tuning/battle_tuning.json"])
	for path in sources:
		assert_str(path).not_contains("res://native/rust/target")
		assert_str(path).not_contains("res://sim/")


func test_a_replay_is_warned_about_only_when_its_build_is_not_this_one() -> void:
	assert_str(SimBuild.compare(SimBuild.id())).is_equal("")
	assert_str(SimBuild.compare("000000000000")).contains("may play out differently")
	assert_str(SimBuild.compare("")).contains("no build stamp")


func test_a_map_in_progress_leaves_the_build_alone() -> void:
	var build := SimBuild.id()
	var path := "res://content/maps/_build_probe.json"
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string("{}\n")
	file.close()
	SimBuild._known = false
	var after := SimBuild.id()
	DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	assert_str(after).is_equal(build)
