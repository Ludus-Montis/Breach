extends GdUnitTestSuite
## BattleTuning, per Decision 123: the battle's tuning numbers live in one data file,
## content/tuning/battle_tuning.json, which sets every one of them - none falls back to the
## script, which only names them. The crate requires every field it names (spec 39 step 2),
## so its names and the script's must be the same.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


func test_the_tuning_file_sets_every_number_it_names() -> void:
	var text := FileAccess.get_file_as_string("res://content/tuning/battle_tuning.json")
	var in_file: Dictionary = JSON.parse_string(text)
	var named := []
	for property in BattleTuning.new().get_property_list():
		var usage: int = property["usage"]
		if usage & PROPERTY_USAGE_SCRIPT_VARIABLE and usage & PROPERTY_USAGE_STORAGE:
			named.append(property["name"])

	assert_array(named).is_not_empty()
	assert_array(in_file.keys()).contains_exactly_in_any_order(named)


func test_the_tuning_in_play_is_the_file() -> void:
	var tuning := BattleTuning.current()

	assert_object(tuning).is_same(BattleTuning.current())
	assert_int(tuning.morale_rear_impact).is_greater(tuning.morale_side_impact)
