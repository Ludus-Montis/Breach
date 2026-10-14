extends GdUnitTestSuite
## Feel-test logs replayed (FormationFieldActions): withdrawals and pursuits on the field's
## roads - turning with a route at its bends (#116), finding a ford again, and a pursuit
## following its quarry down the road it flees by (Decision 113).

const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")


func test_a_withdrawal_from_past_a_bend_turns_with_its_route() -> void:
	# A feel-test log: B, caught on its route's southward leg, withdrew; units past the
	# bend ran on north off the field, the way the leg they were on led, instead of turning
	# west with the route.
	var log := "seed 515557 captain off\n0 pursues off\n120 send B\n318 retreat B"
	var northmost := {"y": INF}  # a lambda captures a local by value
	var field := FormationFieldActions.replay(
		log,
		400,
		func(played, _events):
			for squad in played.sim.squads():
				if squad.faction_id == "player":
					for unit in squad.living():
						northmost["y"] = minf(northmost["y"], ScrumReach.at(squad, unit).y)
	)

	assert_float(northmost["y"]).is_greater(14.0)  # route B runs west along y 21
	assert_bool(field.sim.squads().any(func(s): return s.faction_id == "player")).is_true()


## Feel-test logs: the line pursued down its own route A while its quarry fled by another
## road (Decision 113). Returns [the furthest the line's frame got south, the furthest
## north, whether it ended on route A, back at its post, the routes it travelled].
func _pursuit_roads(log: String, extra: int) -> Array:
	var seen := {"south": -INF, "north": INF, "roads": []}  # a lambda captures by value
	var field := FormationFieldActions.replay(
		log,
		extra,
		func(played, _events):
			var at: Vector2 = played.kingdom_line.position
			seen["south"] = maxf(seen["south"], at.y)
			seen["north"] = minf(seen["north"], at.y)
			for key in played.routes:
				if played.kingdom_line.route == played.routes[key] and key not in seen["roads"]:
					seen["roads"].append(key)
	)
	var line: SkirmishSquad = field.kingdom_line
	var home: bool = line.route == field.routes["A"]
	return [seen["south"], seen["north"], home, line.position, seen["roads"]]


func test_a_pursuit_follows_its_quarry_down_the_road_it_flees_by() -> void:
	var via_c := _pursuit_roads(
		"seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A", 600
	)
	assert_float(via_c[0]).is_greater(50.0)  # down route C, south-west, after A
	assert_bool(via_c[2]).is_true()  # then home to its post, on its own route again
	assert_float(via_c[3].x).is_greater(75.0)

	# Its retreat comes as long after B reaches the line's flank as in the feel test (1529,
	# 15 ticks after): B now pours through the ford in turn, so reaches it later.
	var log := "seed 515557 captain off\n0 pursues off\n120 send B\n318 retreat B\n"
	log += "1314 pursues on\n1326 send B\n1550 retreat B"
	var by_b := _pursuit_roads(log, 700)
	assert_float(by_b[1]).is_less(25.0)  # up route B, north, after B
	assert_array(by_b[4]).contains(["B"])  # its quarry's road, whatever then befalls it


func test_a_unit_fanned_off_its_road_finds_the_ford_again() -> void:
	# A feel-test log: one of B's units, fanned out beside its route as it withdrew, ran
	# along the road's line into the stream beside the ford and stood there until caught.
	var log := "seed 109563 captain off\n88 send B\n304 retreat B"
	var at_stream := {}  # unit id -> ticks spent in the stream's band, withdrawing
	FormationFieldActions.replay(
		log,
		260,
		func(played, _events):
			for squad in played.sim.squads():
				if squad.faction_id != "player" or squad.withdraw.is_empty():
					continue
				for unit in squad.living():
					var x: float = ScrumReach.at(squad, unit).x
					if x >= 27.0 and x <= 32.0:
						at_stream[unit.id] = at_stream.get(unit.id, 0) + 1
	)

	var longest: int = at_stream.values().reduce(func(most, n): return maxi(most, n), 0)
	assert_int(longest).is_less(30)  # it wades the ford and goes on
