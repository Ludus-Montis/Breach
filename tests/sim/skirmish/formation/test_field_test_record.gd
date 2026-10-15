extends GdUnitTestSuite
## The user's feel-test record that showed three bugs, replayed (FormationFieldActions):
## - at B's ford (ticks 600 to 660) no two of B's units make for one point, nor stand on
##   each other (FormationPour, UnitBodies);
## - the line's units chasing B back through the wood (after its retreat at 815) go at the
##   wood's half pace (TerrainStep);
## - A's first group, ordered to retreat at home (550), arrives and holds, never turning
##   (FormationMarch.check_ends);
## and the record plays alike under either engine.

const FormationFieldActions = preload("res://sim/skirmish/formation/formation_field_actions.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")

const RECORD := """record 2 seed 841592 captain off build 789653b8de9e
0 player hurry A on
84 player send A
111 player hurry A off
202 player retreat A
507 player send A
550 player retreat A
567 player send B
815 player retreat B
"""
## B's wave and A's first group, by the squad ids the record gives them.
const B_WAVE := 6
const A_FIRST := 3
## The kingdom's units' march step on open ground (cells a tick at their speed).
const OPEN_STEP := 0.96
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")

static var _seen := {}  # engine -> what its replay saw (_replay)


func after_test() -> void:
	NativeKernels.use(NativeKernels.GDSCRIPT)


## What the record's replay under `engine` shows: B's shared targets and least gap at the
## ford, the kingdom's steps on half-pace ground after B's retreat, A's first group's
## turns after its order, its end state, and a fingerprint of every tick.
func _replay(engine: String) -> Dictionary:
	if _seen.has(engine):
		return _seen[engine]
	NativeKernels.use(engine)
	var seen := {"shared": 0, "least": INF, "wood": [0.0, 0], "turns": 0, "trace": [], "was": {}}
	var field := FormationFieldActions.replay(
		RECORD, 200, func(played, events): _look(played, events, seen)
	)
	seen["first"] = field.sim.squad(A_FIRST)
	seen["trace"] = hash(var_to_str(seen["trace"]))
	_seen[engine] = seen
	return seen


func _look(played, events: Array, seen: Dictionary) -> void:
	var tick: int = played.sim.tick_number()
	for event in events:
		if event["type"] == "turning" and event["squad"] == A_FIRST and tick > 551:
			seen["turns"] += 1
	for squad in played.sim.squads():
		if squad.id == B_WAVE and tick >= 600 and tick <= 660:
			_at_the_ford(squad, played.sim.terrain, seen)
		for unit in squad.living():
			var at: Vector2 = (
				squad.loose[unit.id]["at"]
				if squad.loose.has(unit.id)
				else UnitPlaces.of(squad, unit)
			)
			seen["trace"].append(at)
			var was = seen["was"].get(unit.id)
			seen["was"][unit.id] = at
			if squad.faction_id != "the_kingdom" or tick <= 816 or was == null or at == was:
				continue
			if played.sim.terrain.factor(unit, was, was + (at - was).normalized()) == 0.5:
				seen["wood"][0] += was.distance_to(at)
				seen["wood"][1] += 1


func _at_the_ford(squad: SkirmishSquad, terrain, seen: Dictionary) -> void:
	var units := squad.living()
	var aims := {}
	for unit in units:
		if not squad.loose.has(unit.id) and not squad.fleeing.has(unit.id):
			var aim := FormationWalk.target_of(squad, unit, terrain)
			seen["shared"] += 1 if aims.has(aim) else 0
			aims[aim] = true
	for i in units.size():
		for j in range(i + 1, units.size()):
			var apart: float = UnitPlaces.of(squad, units[i]).distance_to(
				UnitPlaces.of(squad, units[j])
			)
			seen["least"] = minf(seen["least"], apart)


func test_at_bs_ford_no_two_units_share_a_target_or_stand_on_each_other() -> void:
	var seen := _replay(NativeKernels.GDSCRIPT)
	assert_int(seen["shared"]).is_equal(0)
	assert_float(seen["least"]).is_greater(0.5)


func test_the_line_chasing_through_the_wood_goes_at_its_half_pace() -> void:
	var seen := _replay(NativeKernels.GDSCRIPT)
	assert_int(seen["wood"][1]).is_greater(50)
	assert_float(seen["wood"][0] / seen["wood"][1]).is_less(OPEN_STEP * 0.5 + 0.05)


func test_as_first_group_ordered_home_while_home_holds_there() -> void:
	var seen := _replay(NativeKernels.GDSCRIPT)
	var first: SkirmishSquad = seen["first"]
	assert_int(seen["turns"]).is_equal(0)
	assert_int(first.order).is_equal(SkirmishUnit.Order.HOLD)
	assert_float(first.front_distance).is_equal(first.home_distance)
	assert_bool(first.loose.is_empty()).is_true()


func test_the_record_plays_alike_under_either_engine() -> void:
	if not NativeKernels.available(NativeKernels.RUST):
		print("test_field_test_record: no native kernel built - native case skipped")
		return
	var reference := _replay(NativeKernels.GDSCRIPT)
	var native := _replay(NativeKernels.RUST)
	assert_int(native["trace"]).is_equal(reference["trace"])
