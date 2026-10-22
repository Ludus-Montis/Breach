extends GdUnitTestSuite
## CrateContent (spec 39 step 2, Decision 153): GDScript's definitions filled from the
## content the Rust crate loaded, the JSON under content/ - never from a .tres.

const KINDS := ["units", "weapons", "armour", "tuning"]


func test_a_fresh_definition_holds_the_crates_defaults() -> void:
	var defaults := CrateContent.defaults()
	assert_dict(defaults).is_not_empty()  # the core is built
	var unit_fields: Dictionary = defaults["unit"].duplicate()
	unit_fields.erase("items")

	_assert_same_fields(CrateContent.fill(UnitDef.new(), unit_fields), UnitDef.new())
	_assert_same_fields(CrateContent.fill(WeaponDef.new(), defaults["weapon"]), WeaponDef.new())
	_assert_same_fields(CrateContent.fill(ArmourDef.new(), defaults["armour"]), ArmourDef.new())


func test_a_unit_is_filled_from_its_file() -> void:
	var captain := CrateContent.unit("kingdom_captain")

	assert_int(captain.hp).is_equal(30)
	assert_float(captain.speed).is_equal(0.8)
	assert_int(captain.preferred_position).is_equal(UnitDef.Position.MID)
	assert_array(captain.tags).contains_exactly(["human", "martial", "leader"])
	assert_dict(captain.proficiencies).is_equal({"polearm": 2})


func test_a_units_items_are_the_shared_definitions() -> void:
	var grem := CrateContent.unit("grem")

	assert_int(grem.items.size()).is_equal(2)
	assert_object(grem.items[0]).is_same(CrateContent.weapon("grem_bite"))
	assert_object(grem).is_same(CrateContent.unit("grem"))
	assert_bool(CrateContent.weapon("brute_fists").innate).is_true()
	assert_dict(CrateContent.weapon("brute_fists").traits).is_equal({"siege": 1})


func test_a_list_of_reals_keeps_its_infinity() -> void:
	var steps: Array[float] = CrateContent.tuning().discipline_leash_steps

	assert_float(steps[steps.size() - 1]).is_equal(INF)


func test_no_tres_definition_is_left_or_read() -> void:
	for kind: String in KINDS:
		if not DirAccess.dir_exists_absolute("res://content/" + kind):
			continue
		for file in DirAccess.get_files_at("res://content/" + kind):
			assert_str(file.get_extension()).is_not_equal("tres")
	var pattern := RegEx.create_from_string("content/(%s)/\\w+\\.tres" % "|".join(KINDS))
	for path in _sources("res://", []):
		var found := pattern.search(FileAccess.get_file_as_string(path))
		(
			assert_object(found)
			. override_failure_message("%s reads a .tres definition" % path)
			. is_null()
		)


## Every exported field of `filled` equals the same field of `fresh`.
func _assert_same_fields(filled: Object, fresh: Object) -> void:
	for property in fresh.get_property_list():
		var usage: int = property["usage"]
		if usage & PROPERTY_USAGE_SCRIPT_VARIABLE and usage & PROPERTY_USAGE_STORAGE:
			var name: String = property["name"]
			(
				assert_that(filled.get(name))
				. override_failure_message("%s: %s" % [fresh.get_script().resource_path, name])
				. is_equal(fresh.get(name))
			)


## The .gd, .tscn, .py and .sh files under `dir`, skipping addons, .godot and native.
func _sources(dir: String, found: Array) -> Array:
	for sub in DirAccess.get_directories_at(dir):
		if not sub.begins_with(".") and sub not in ["addons", "native", "node_modules"]:
			_sources(dir.path_join(sub), found)
	for file in DirAccess.get_files_at(dir):
		if file.get_extension() in ["gd", "tscn", "py", "sh"]:
			found.append(dir.path_join(file))
	return found
