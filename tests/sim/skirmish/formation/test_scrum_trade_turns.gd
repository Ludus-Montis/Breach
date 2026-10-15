extends GdUnitTestSuite
## Trading places in turn (Decision 110): each stalled unit, in its draw's order, finds the
## places as the trades before it left them - the same trades as looking through every
## living friend for each one.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumTrade = preload("res://sim/skirmish/formation/scrum_trade.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitPlaces = preload("res://tests/sim/skirmish/formation/unit_places.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

var _places := UnitPlaces.new()


func before_test() -> void:
	_places = UnitPlaces.new()


func _unit(unit_id: int, rank: int, kind: UnitDef) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.hp = 10
	unit.rank = rank
	unit.definition = kind
	return unit


## A squad of like units, `width` wide, its places laid out; some loose and held, scattered
## by a seeded generator.
func _crowd(seed_value: int, kind: UnitDef) -> SkirmishSquad:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var members: Array[SkirmishUnit] = []
	for index in range(24):
		var unit := _unit(index + 1, index / 4, kind)
		unit.column = index % 4
		members.append(unit)
	var squad := SkirmishSquad.new(1, "player", 1, 0.0, 4, members, null, _places.store)
	for unit in members:
		_places.stand(unit, ScrumStance.anchor(squad, unit))
		if rng.randf() < 0.5:
			var at := _places.at(unit) + Vector2(rng.randf_range(-5, 5), rng.randf_range(-5, 5))
			SquadEntries.loosen(squad, unit, {"unit": unit, "goal": null, "next": at}, at)
			squad.loose[unit.id]["stuck"] = BattleTuning.current().scrum_trade_seconds
	return squad


func test_trades_in_turn_find_the_places_earlier_trades_left() -> void:
	var kind := UnitDef.new()
	for seed_value in range(1, 9):
		var squad := _crowd(seed_value, kind)
		var reference := _crowd(seed_value, kind)

		ScrumTrade.trade(squad, seed_value)
		_trade_looking_at_every_friend(reference, seed_value)

		for index in range(squad.units.size()):
			var unit: SkirmishUnit = squad.units[index]
			var twin: SkirmishUnit = reference.units[index]
			assert_vector(Vector2(unit.rank, unit.column)).is_equal(Vector2(twin.rank, twin.column))
		assert_array(squad.loose.keys()).contains_exactly_in_any_order(reference.loose.keys())


## The trades as made by looking through every living friend for each stalled unit.
func _trade_looking_at_every_friend(squad: SkirmishSquad, fight_seed: int) -> void:
	var stalled: Array = squad.loose.values().map(func(entry): return entry["unit"])
	stalled.sort_custom(
		func(a, b): return ScrumContest.draw(a, fight_seed) < ScrumContest.draw(b, fight_seed)
	)
	for unit in stalled:
		var at: Vector2 = SquadEntries.loose_at(squad, unit)
		var own := ScrumStance.anchor(squad, unit).distance_to(at)
		var best: SkirmishUnit = null
		var best_key := []
		for friend in squad.living():
			var there := ScrumStance.anchor(squad, friend).distance_to(at)
			if friend == unit or there >= own - ScrumTrade.PROGRESS:
				continue
			var key := [snappedf(there, 0.000001), ScrumContest.draw(friend, fight_seed)]
			if best_key.is_empty() or key < best_key:
				best = friend
				best_key = key
		if best == null:
			SquadEntries.unloose(squad, unit.id)
			continue
		var place := Vector2i(unit.rank, unit.column)
		unit.rank = best.rank
		unit.column = best.column
		best.rank = place.x
		best.column = place.y
		if not squad.loose.has(best.id):
			SquadEntries.loosen(squad, best, {"unit": best, "goal": null}, _places.at(best))
