extends GdUnitTestSuite
## CrateContent (spec 39 step 2, Decision 153): GDScript's definitions filled from the
## content the Rust crate loaded, the JSON under content/ - never from a .tres.

const KINDS := ["units", "weapons", "armour", "tuning", "terrain", "maps"]
const MAPS := ["demo_map", "p_f_F_c", "skirmish_p_c", "skirmish_two_lanes"]


func test_a_fresh_definition_holds_the_crates_defaults() -> void:
	var defaults := CrateContent.defaults()
	assert_dict(defaults).is_not_empty()  # the core is built
	for class_id: String in defaults:
		_assert_same_fields(defaults[class_id], _script_named(class_id).new())


func test_the_terrain_library_is_filled_from_its_file() -> void:
	var library := CrateContent.terrain_library()
	var fields := library.terrain("FIELDS")

	assert_str(fields.display_name).is_equal("Fields")
	assert_that(fields.color).is_equal(Color("#8f9a4f"))
	assert_float(fields.move_cost).is_equal(1.0)
	assert_int(fields.relief_scale).is_equal(12)
	assert_object(fields.strata[2]).is_instanceof(StratumDef)
	assert_str(fields.strata[2].material_id).is_equal("ROCK")
	assert_float(fields.liquids[0].surface_chance).is_equal(0.02)
	assert_int(library.material("TIMBER").heat_transitions[0].threshold).is_equal(3)
	assert_bool(library.material("WATER").is_liquid()).is_true()
	assert_array(library.validate()).is_empty()


func test_every_layout_shares_the_one_terrain_library() -> void:
	var library := CrateContent.terrain_library()

	assert_object(MapLayoutDef.new().terrain_library).is_same(library)
	assert_object(CrateContent.map("demo_map").layout.terrain_library).is_same(library)


func test_a_map_is_filled_from_its_file() -> void:
	var map := CrateContent.map("demo_map")
	var lane: LaneDef = map.lanes[0]

	assert_array(lane.nodes.map(func(node): return node.id)).is_equal(["p", "f", "F", "c"])
	assert_array(map.off_lane_nodes.map(func(node): return node.id)).is_equal(["s2", "w1"])
	var fort: NodeDef = lane.nodes[2]
	assert_int(fort.node_type).is_equal(NodeDef.NodeType.FORT)
	assert_object(fort.position).is_equal(Vector2(416, 160))
	assert_object(fort.tile).is_equal(Vector2i(6, 2))
	assert_object(fort.garrison_units[0]).is_instanceof(GarrisonUnitDef)
	assert_array(fort.garrison_units[0].patrol_route).is_equal(["s2"])
	assert_int(map.faction_relations[0].stance).is_equal(FactionRelationDef.Stance.HOSTILE)
	assert_object(map.layout.tile_at(Vector2i(2, 3))).is_not_null()
	assert_object(map.layout.roads[0].a).is_equal(Vector2i(1, 2))
	assert_object(map).is_same(CrateContent.map("demo_map"))


func test_a_node_on_two_lanes_is_one_node() -> void:
	var map := CrateContent.map("skirmish_two_lanes")

	assert_object(map.lanes[0].nodes[0]).is_same(map.lanes[1].nodes[0])


func test_every_checked_in_map_is_there() -> void:
	var files := Array(DirAccess.get_files_at("res://content/maps"))
	assert_array(files).contains_exactly_in_any_order(MAPS.map(func(m): return m + ".json"))
	for name: String in MAPS:
		assert_object(CrateContent.map(name)).override_failure_message(name).is_not_null()


func test_a_map_written_as_json_reads_back_the_same() -> void:
	var map := _small_map()
	var path := OS.get_user_data_dir().path_join("test_map_json.json")

	assert_str(MapFiles.write(map, path)).is_empty()
	var read := MapFiles.read(path)

	assert_dict(read).contains_keys(["map"])
	_assert_same_tree(read["map"], map, "map")


func test_a_map_that_breaks_a_rule_isnt_written() -> void:
	var map := _small_map()
	map.lanes[0].nodes[1].garrison = -1
	var path := OS.get_user_data_dir().path_join("test_map_json_bad.json")

	assert_str(MapFiles.write(map, path)).contains("garrison must be >= 0, got -1")
	assert_bool(FileAccess.file_exists(path)).is_false()


## A valid map with a plan, a subnode, an off-lane node and a layout with a bridge.
func _small_map() -> MapDef:
	var map := MapDef.new()
	map.tick_duration_seconds = 1.5
	var home := NodeDef.new()
	home.id = "p"
	home.position = Vector2(96.5, 160)
	home.tile = Vector2i(1, 2)
	home.plan = StructurePlanDef.new()
	home.plan.solid_cells[Vector3i(3, 4, 0)] = "ROCK"
	home.plan.faces.append(StructureFaceDef.make(Vector3i(1, 1, 0), "west", "TIMBER", 2))
	home.plan.dug.append(Vector3i(2, 2, -1))
	home.plan.loads[Vector3i(1, 1, 1)] = 40
	var well := SubnodeDef.new()
	well.id = "well"
	well.capture_cells.append(Vector2i(5, 6))
	well.zone_cells.assign([Vector2i(0, 0), Vector2i(5, 6)])
	home.subnodes.append(well)
	var core := NodeDef.new()
	core.id = "c"
	core.node_type = NodeDef.NodeType.NEUTRAL
	var lane := LaneDef.new()
	lane.id = "to_c"
	lane.nodes.assign([home, core])
	map.lanes.append(lane)
	var aside := NodeDef.new()
	aside.id = "w"
	map.off_lane_nodes.append(aside)
	map.layout = MapLayoutDef.new()
	map.layout.cols = 4
	map.layout.rows = 4
	map.layout.default_terrain_id = "FIELDS"
	var tile := TileDef.new()
	tile.terrain_id = "WATER"
	tile.bridge = BridgeDef.new()
	map.layout.tiles.append(tile)
	return map


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
	var pattern := RegEx.create_from_string('content/(%s)/[^"\\n]*\\.tres' % "|".join(KINDS))
	for path in _sources("res://", []):
		var found := pattern.search(FileAccess.get_file_as_string(path))
		(
			assert_object(found)
			. override_failure_message("%s reads a .tres definition" % path)
			. is_null()
		)


## The script declaring `class_name class_id`.
func _script_named(class_id: String) -> Script:
	for known in ProjectSettings.get_global_class_list():
		if known["class"] == class_id:
			return load(known["path"])
	return null


## Every exported field of `read` equals `made`'s, through its definitions and lists of them.
func _assert_same_tree(read: Variant, made: Variant, at: String) -> void:
	if made is Object and made != null:
		assert_object(read).override_failure_message(at).is_not_null()
		for property in made.get_property_list():
			var usage: int = property["usage"]
			if usage & PROPERTY_USAGE_SCRIPT_VARIABLE and usage & PROPERTY_USAGE_STORAGE:
				var name: String = property["name"]
				_assert_same_tree(read.get(name), made.get(name), at + "." + name)
	elif made is Array:
		assert_int(read.size()).override_failure_message(at).is_equal(made.size())
		for index in made.size():
			_assert_same_tree(read[index], made[index], "%s[%d]" % [at, index])
	else:
		assert_that(read).override_failure_message(at).is_equal(made)


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
