extends GdUnitTestSuite
## The ground made in the crate from a map (specs/39-simulation-in-rust.md step 8c-v, native
## ground/map.rs): FormationField's ground, made by `TerrainGround.map(content, "field")`, is
## cell for cell the ground FormationField._ground paints - its costs, heights, depths,
## climbs, flows, sight and the fastest ground painted.

const FormationField = preload("res://sim/skirmish/formation/formation_field.gd")


func _made(map: String) -> Object:
	var ground: Object = ClassDB.instantiate("TerrainGround")
	return ground if ground.map(CrateContent.store(), map) else null


func test_the_field_s_ground_made_in_the_crate_is_the_one_gdscript_paints() -> void:
	var painted: PackedByteArray = FormationField._ground().core.snapshot()
	var made: Object = _made("field")
	assert_object(made).is_not_null()
	assert_bool(made.snapshot() == painted).is_true()
	made.paint(Rect2i(0, 0, 1, 1), {"height": 1})
	var bites := "a cell painted on one side differs"
	assert_bool(made.snapshot() == painted).override_failure_message(bites).is_false()


func test_an_unknown_map_makes_no_ground() -> void:
	assert_object(_made("nowhere")).is_null()
