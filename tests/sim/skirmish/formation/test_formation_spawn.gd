extends GdUnitTestSuite
## A squad spawned in the crate from its units' definitions (specs/39-simulation-in-rust.md
## step 8c-iv, native spawn.rs): the crate's whole state after its spawns is the state
## FormationSimulation.spawn_squad leaves - the ids, squads, commands, routes and their
## codes, each unit's sheet, words, objects, place, hp, stamina and pace, each squad's
## facing and morale, and its units stood on their places.

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

const ROUTE := 9.0
const TICK := 0.1
const COMBAT_WIDTH := 5


## Three squads, as [width, unit ids, faction, at the player's end, wait ticks, on its own
## route]: grems led by a chieftain at the player's end, then militia under a captain on a
## route of their own at the kingdom's, then grems on the lane's route again.
func _sides() -> Array:
	var player := ["grem_chieftain", "grem", "grem_brute", "grem", "grem_spitter"]
	var kingdom := ["kingdom_captain", "kingdom_militia", "kingdom_militia"]
	return [
		[3, player, "player", true, 2, false],
		[2, kingdom, "the_kingdom", false, 0, true],
		[1, ["grem", "grem"], "player", true, 0, false],
	]


## `ids`' definitions at their places, `width` abreast.
func _placements(ids: Array, width: int) -> Array:
	var out := []
	for index in ids.size():
		out.append([CrateContent.unit(ids[index]), Vector2i(index / width, index % width)])
	return out


## A bent route of its own, 7 tiles.
func _bent() -> FormationRoute:
	var points := PackedVector2Array([Vector2(0, 0), Vector2(256, 64), Vector2(448, 64)])
	return FormationRoute.new(points, 1.5)


func _sim() -> FormationSimulation:
	var sim := FormationSimulation.new(ROUTE, TICK)
	sim.combat_width = COMBAT_WIDTH
	return sim


## The crate's state after GDScript spawns the sides.
func _by_gdscript() -> PackedByteArray:
	var sim := _sim()
	var bent := _bent()
	for side in _sides():
		var route: FormationRoute = bent if side[5] else null
		sim.spawn_squad(side[0], _placements(side[1], side[0]), side[2], side[3], side[4], route)
	return sim._field.snapshot(0, 0)


## The crate's state after it spawns the sides itself.
func _by_crate() -> PackedByteArray:
	var sim := _sim()
	var bent := _bent()
	for side in _sides():
		var route: FormationRoute = bent if side[5] else sim._route
		var codes := PackedInt64Array()
		var places := PackedInt32Array()
		for placement in _placements(side[1], side[0]):
			codes.append(sim.store.definition_code(placement[0]))
			places.append_array([placement[1].x, placement[1].y])
		var ints := PackedInt64Array([side[0], int(side[3]), side[4], sim.combat_width])
		var slot: int = sim._field.spawn_squad(
			CrateContent.store(),
			PackedStringArray(side[1]),
			codes,
			places,
			side[2],
			ints,
			route.points(),
			route.corridor_half_width,
			route.get_instance_id()
		)
		assert_int(slot).is_greater_equal(0)
	return sim._field.snapshot(0, 0)


func test_the_crate_spawns_each_squad_as_formation_simulation_spawn_squad_does() -> void:
	var by_gdscript := _by_gdscript()
	assert_int(by_gdscript.size()).is_greater(1000)
	var differs := "the crate's state after its spawns differs from GDScript's"
	assert_bool(_by_crate() == by_gdscript).override_failure_message(differs).is_true()


func test_no_squad_is_spawned_for_a_definition_that_isnt_there() -> void:
	var sim := _sim()
	var points := sim._route.points()
	var slot: int = sim._field.spawn_squad(
		CrateContent.store(),
		PackedStringArray(["no_such_unit"]),
		PackedInt64Array([0]),
		PackedInt32Array([0, 0]),
		"player",
		PackedInt64Array([1, 1, 0, 0]),
		points,
		0.0,
		0
	)
	assert_int(slot).is_equal(-1)
	assert_int(sim._field.squad_count()).is_equal(0)
