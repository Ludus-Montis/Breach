extends GdUnitTestSuite
## Every tool under tools/ loads, so a change to the code a tool calls fails its own PR,
## not a bench run weeks later (#601: tick_phases stopped parsing when step()'s _move
## gained its roster).


func test_every_tool_loads() -> void:
	var broken := []
	for file in DirAccess.get_files_at("res://tools"):
		if not file.ends_with(".gd"):
			continue
		var script: GDScript = ResourceLoader.load(
			"res://tools/" + file, "", ResourceLoader.CACHE_MODE_IGNORE
		)
		if script == null or not script.can_instantiate():
			broken.append(file)
	assert_array(broken).is_empty()
