extends GdUnitTestSuite
## BlowLanding, per Decision 118 (spec 28 part 4): with rolls on, a tick's blows are each
## rolled against their targets - a wavering formation's land worse, a unit fought by two
## foes is struck more surely - and the same seed lands them the same; with rolls off,
## every blow is a plain hit. Each blow lands with the arms the crate holds for its units
## (spec 39 step 8b5-xviii).

const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const WeaponDef = preload("res://content/definitions/weapon_def.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const NativeKernels = preload("res://sim/skirmish/formation/native_kernels.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

const TICK := 0.1
const LANDED := ["grazed", "hit", "critical"]


func _def() -> UnitDef:
	var unit_def := UnitDef.new()
	unit_def.hp = 100000
	unit_def.speed = 1.0
	unit_def.items = [WeaponDef.innate_weapon(4)]
	return unit_def


## A duel of `attackers` player units against one kingdom unit holding on flat ground,
## fought `ticks` ticks: the hit events, the kingdom's morale held at `morale` if >= 0.
func _duel(attackers: int, rolls: bool, morale: int = -1, ticks: int = 1200) -> Array:
	var sim := FormationSimulation.new(1.0, TICK)
	sim.blow_rolls = rolls
	sim.fight_seed = 11
	var east := FormationRoute.new(PackedVector2Array([Vector2(0, 10), Vector2(64, 10)]))
	var west := FormationRoute.new(PackedVector2Array([Vector2(40, 10), Vector2(0, 10)]))
	var row := []
	for column in range(attackers):
		row.append([_def(), Vector2i(0, column)])
	sim.spawn_squad(attackers, row, "player", true, 0, east)
	var line := sim.spawn_squad(1, [[_def(), Vector2i(0, 0)]], "the_kingdom", true, 0, west)
	sim.order(line.id, SkirmishUnit.Order.HOLD)
	var hits := []
	for _i in range(ticks):
		if morale >= 0:
			line.morale = morale
		hits.append_array(sim.step().filter(func(e): return e["type"] == "hit"))
	return hits


func _landed_share(hits: Array, faction: String) -> float:
	var own := hits.filter(func(h): return h["faction"] == faction)
	return float(own.filter(func(h): return LANDED.has(h["blow"])).size()) / maxi(1, own.size())


func test_without_rolls_every_blow_is_a_plain_hit() -> void:
	var hits := _duel(1, false, -1, 300)

	assert_bool(hits.is_empty()).is_false()
	for hit in hits:
		assert_str(hit["blow"]).is_equal("hit")
		assert_int(hit["dmg"]).is_equal(4)


func test_a_wavering_formation_lands_fewer_of_its_blows() -> void:
	var steady := _landed_share(_duel(1, true), "the_kingdom")
	var wavering := _landed_share(_duel(1, true, 24), "the_kingdom")

	assert_float(wavering).is_less(steady - 0.05)


func test_a_unit_fought_by_two_is_struck_more_surely() -> void:
	var alone := _landed_share(_duel(1, true), "player")
	var pressed := _landed_share(_duel(2, true), "player")

	assert_float(pressed).is_greater(alone)


func test_the_same_seed_lands_the_same_blows() -> void:
	assert_array(_duel(1, true, -1, 300)).is_equal(_duel(1, true, -1, 300))


## A squad of one unit of `faction` in `store`.
func _one(unit_id: int, faction: String, store: UnitStore) -> SkirmishSquad:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = faction
	unit.max_hp = 10
	var typed: Array[SkirmishUnit] = [unit]
	return SkirmishSquad.new(unit_id, faction, 1, 0.0, 1, typed, null, store)


func test_a_blow_lands_with_the_arms_the_crate_holds_for_its_units() -> void:
	var store := NativeKernels.unit_store(null)
	var ours := _one(1, "player", store)
	var theirs := _one(2, "the_kingdom", store)
	var striker: SkirmishUnit = ours.units[0]
	var target: SkirmishUnit = theirs.units[0]
	striker.melee_parts = [[6.0, "fire", true]]
	target.weaknesses = ["fire"]
	target.ward = 1
	var blows := [[striker, target, 6, false]]

	BlowLanding.land(blows, [], [ours, theirs], null, [])

	var weak := BattleTuning.current().damage_weak
	assert_array(blows[0].slice(2)).is_equal([maxi(1, roundi(6 * weak - 1)), false, "hit"])
