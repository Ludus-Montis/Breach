extends GdUnitTestSuite
## The registry, per Decision 128: every tag, trait and status defined once with its tags,
## a trait's tags saying where it may sit; and all content - units, items, materials -
## and every trait the simulation reads, held to it.

const ContentRegistry = preload("res://content/definitions/content_registry.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const ItemDef = preload("res://content/definitions/item_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const ArmourDef = preload("res://content/definitions/armour_def.gd")

const UNITS := "res://content/units/"
const WEAPONS := "res://content/weapons/"
const CODE := ["res://sim/", "res://content/definitions/", "res://content/import/"]


func test_the_registry_holds_together() -> void:
	var registry := ContentRegistry.current()
	assert_bool(registry["traits"].is_empty()).is_false()
	for trait_id in registry["traits"]:
		var entry: Dictionary = registry["traits"][trait_id]
		assert_array(ContentRegistry.tag_errors(entry["tags"], "trait", trait_id)).is_empty()
		assert_array(ContentRegistry.targets(trait_id)).is_not_empty()
		assert_str(entry.get("description", "")).is_not_empty()
	for status_id in registry["statuses"]:
		var tags: Array = registry["statuses"][status_id]["tags"]
		assert_array(ContentRegistry.tag_errors(tags, "status", status_id)).is_empty()


func test_an_implicit_trait_is_every_units_at_its_default_level() -> void:
	var implicit := ContentRegistry.implicit()

	assert_dict(implicit).contains_key_value("climber", 0).contains_key_value("swimmer", 0)
	for trait_id in implicit:
		assert_int(implicit[trait_id]).is_greater_equal(0)
		assert_bool(ContentRegistry.targets(trait_id).has("unit")).is_true()
	var plain := UnitDef.new()
	assert_int(plain.trait_level("climber")).is_equal(implicit["climber"])
	plain.traits = {"climber": 2}
	assert_int(plain.trait_level("climber")).is_equal(2)


func test_a_trait_sits_only_where_its_tags_agree() -> void:
	assert_array(ContentRegistry.targets("cunning")).contains_exactly_in_any_order(
		["unit", "leader"]
	)
	assert_array(ContentRegistry.targets("messenger")).contains_exactly(["leader"])
	assert_array(ContentRegistry.targets("siege")).contains_exactly(["weapon"])
	assert_array(ContentRegistry.trait_errors({"cunning": 1}, ["weapon"], "a sword")).is_not_empty()
	assert_array(ContentRegistry.trait_errors({"cunning": 1}, ["unit"], "a grem")).is_empty()
	assert_array(ContentRegistry.trait_errors({"guile": 1}, ["unit"], "a grem")).is_not_empty()
	assert_array(ContentRegistry.tag_errors(["polearm"], "unit", "a grem")).is_not_empty()


func test_every_unit_and_item_in_play_keeps_to_it() -> void:
	for file in DirAccess.get_files_at(UNITS):
		if file.ends_with(".tres"):
			var unit: UnitDef = load(UNITS + file)
			var kinds := ["unit", "leader"] if unit.leadership > 0 else ["unit"]
			assert_array(ContentRegistry.trait_errors(unit.traits, kinds, file)).is_empty()
			assert_array(ContentRegistry.tag_errors(unit.tags, "unit", file)).is_empty()
			for item in unit.items:
				_check_item(item, file)


func test_the_materials_keep_to_it() -> void:
	var library := CrateContent.terrain_library()
	var checked := 0
	for material in library.materials:
		(
			assert_array(ContentRegistry.trait_errors(material.traits, ["material"], "material"))
			. is_empty()
		)
		checked += 1
	assert_int(checked).is_greater(0)


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


func _check_item(item: ItemDef, owner: String) -> void:
	var kind := "weapon" if item is WeaponDef else ("armour" if item is ArmourDef else "tool")
	var what := "%s's %s" % [owner, item.item_name]
	assert_array(ContentRegistry.trait_errors(item.traits, [kind], what)).is_empty()
	assert_array(ContentRegistry.trait_errors(item.grants, ["unit"], what)).is_empty()
	assert_array(ContentRegistry.tag_errors(item.tags, "item", what)).is_empty()


func _scan(dir: String, read: RegEx, found: Dictionary) -> void:
	for file in DirAccess.get_files_at(dir):
		if file.ends_with(".gd"):
			for match in read.search_all(FileAccess.get_file_as_string(dir + file)):
				found[match.get_string(1)] = dir + file
	for sub in DirAccess.get_directories_at(dir):
		_scan(dir + sub + "/", read, found)
