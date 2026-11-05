extends GdUnitTestSuite
## The registry, per Decision 128: the implicit traits GDScript reads, and every trait the
## code reads registered. The registry's own rules, and the content held to it, are the
## crate's (`native/rust/src/content/checks.rs`, spec 39 step 9e-i).

const ContentRegistry = preload("res://content/definitions/content_registry.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")

## Where the code that reads traits lives: the crate (the rules, since spec 39 step 9d-iv) and
## the definitions.
const CODE := ["res://native/rust/src/", "res://content/definitions/", "res://content/import/"]


func test_an_implicit_trait_is_every_units_at_its_default_level() -> void:
	var implicit := ContentRegistry.implicit()

	assert_dict(implicit).contains_key_value("climber", 0).contains_key_value("swimmer", 0)
	for trait_id in implicit:
		assert_int(implicit[trait_id]).is_greater_equal(0)
	var plain := UnitDef.new()
	assert_int(plain.trait_level("climber")).is_equal(implicit["climber"])
	plain.traits = {"climber": 2}
	assert_int(plain.trait_level("climber")).is_equal(2)


func test_every_trait_the_code_reads_is_registered() -> void:
	var read := RegEx.create_from_string(
		'(?:traits\\.get|trait_level|_marches\\(squad,)\\s*\\(?\\s*"([a-z_]+)"'
	)
	var found := {}
	for root in CODE:
		_scan(root, read, found)
	assert_int(found.size()).is_greater(5)
	for trait_id in found:
		(
			assert_bool(ContentRegistry.current()["traits"].has(trait_id))
			. override_failure_message(
				"%s reads trait '%s', which isn't registered" % [found[trait_id], trait_id]
			)
			. is_true()
		)


func _scan(dir: String, read: RegEx, found: Dictionary) -> void:
	for file in DirAccess.get_files_at(dir):
		if file.ends_with(".gd") or file.ends_with(".rs"):
			for match in read.search_all(FileAccess.get_file_as_string(dir + file)):
				found[match.get_string(1)] = dir + file
	for sub in DirAccess.get_directories_at(dir):
		_scan(dir + sub + "/", read, found)
